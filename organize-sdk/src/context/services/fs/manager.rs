use crate::{
	context::{
		services::{fs::locker::Locker, task_manager::ProgressHandle},
		ExecutionContext,
	},
	engine::rule::RuleBuilder,
	error::Error,
	plugins::storage::{BackendType, StorageProvider},
	resource::{FileState, Resource},
	stdx::path::PathExt,
	templates::template::{Template, TemplateString},
};
use anyhow::Result;
use bytes::Bytes;
use futures::{self, StreamExt, TryStreamExt};
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	sync::Arc,
};
use tempfile::NamedTempFile;
use tokio::io::AsyncWriteExt;

fn default_host() -> TemplateString {
	TemplateString("file".to_string())
}

const COPY_THRESHOLD: u64 = 1024 * 1024 * 1024; // 1GB

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct DestinationBuilder {
	pub folder: TemplateString,
	pub filename: Option<TemplateString>,
	#[serde(default = "default_host")]
	pub host: TemplateString,
}
impl DestinationBuilder {
	/// Compiles the raw DestinationBuilder into an executable Destination.
	pub async fn build(&self, ctx: &ExecutionContext) -> Result<Destination, Error> {
		let folder = ctx.services.template_compiler.compile_template(&self.folder)?;
		let filename = self
			.filename
			.clone()
			.map(|f| ctx.services.template_compiler.compile_template(&f))
			.transpose()?; // This elegantly handles the Option<Result<T, E>>
		let host = ctx.services.template_compiler.compile_template(&self.host)?.render(ctx).await?;
		Ok(Destination { folder, filename, host })
	}
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Destination {
	pub folder: Template,
	pub filename: Option<Template>,
	pub host: String,
}

impl Destination {
	pub async fn resolve(&self, ctx: &ExecutionContext) -> Result<PathBuf> {
		let mut path = PathBuf::from(self.folder.render(ctx).await?);
		if let Some(filename_template) = &self.filename {
			let filename = filename_template.render(ctx).await?;
			path.push(filename);
		}
		// A placeholder for the original filename if `filename` is not provided.
		// This would need access to the resource from the context.
		else if let Ok(resource) = ctx.scope.resource() {
			if let Some(name) = resource.path.file_name() {
				path.push(name);
			}
		}
		Ok(path)
	}
}

#[derive(Debug, Clone)]
pub struct FileSystemManager {
	pub locker: Locker,
	pub resources: Cache<PathBuf, Arc<Resource>>,
	pub tracked_files: Cache<PathBuf, FileState>,
	pub backends: HashMap<String, Arc<dyn StorageProvider>>,
}

impl FileSystemManager {
	pub fn new(rule: &RuleBuilder) -> Self {
		let mut backends: HashMap<String, Arc<dyn StorageProvider + 'static>> = rule
			.connections
			.iter()
			.map(|(k, v)| (k.clone(), Arc::from(v.clone())))
			.collect();

		backends.insert(
			"file".to_string(),
			serde_json::from_value::<Arc<dyn StorageProvider>>(json!({ "type": "local" })).expect("missing local file system provider"),
		);

		Self {
			locker: Locker::default(),
			resources: Cache::new(10_000),
			tracked_files: Cache::new(10_000),
			backends,
		}
	}

	pub fn get_provider(&self, host: &str) -> Result<Arc<dyn StorageProvider>> {
		self.backends
			.get(host)
			.cloned()
			.ok_or_else(|| anyhow::anyhow!("No provider found for host: {}", host))
	}

	pub async fn ensure_parent_dir_exists(&self, path: &Path) -> std::io::Result<()> {
		if let Some(parent) = path.parent() {
			if !tokio::fs::try_exists(parent).await.unwrap_or(false) {
				tokio::fs::create_dir_all(parent).await?;
			}
		}
		Ok(())
	}

	async fn remote_to_local_copy(
		&self,
		progress: Arc<ProgressHandle>,
		from: &Resource,
		to: &Resource,
		ctx: &ExecutionContext,
	) -> Result<String, Error> {
		progress
			.clone()
			.new_long_step(&format!("Downloading {}", from.as_path().display()), ctx, async {
				let mut stream = from.backend.download(from.as_path());
				let temp_file = NamedTempFile::new()?;
				let mut writer = tokio::fs::File::create(temp_file.path()).await?;

				while let Ok(Some(chunk)) = stream.try_next().await {
					writer.write_all(&chunk).await?;
					progress.increment(chunk.len() as u64);
				}

				to.backend.rename(&temp_file.into_temp_path(), to.as_path()).await
			})
			.await?;

		Ok(format!("Downloaded {} -> {}", from.as_path().display(), to.as_path().display()))
	}

	async fn any_to_remote_copy(
		&self,
		task: Arc<ProgressHandle>,
		from: &Resource,
		to: &Resource,
		description: String,
		ctx: &ExecutionContext,
	) -> Result<String, Error> {
		task.clone()
			.new_long_step(&description, ctx, async {
				let download = from.backend.download(from.as_path()).map(move |chunk: Result<Bytes, Error>| {
					if let Ok(chunk) = &chunk {
						task.increment(chunk.len() as u64);
					}
					chunk
				});
				to.backend.upload(to.as_path(), Box::pin(download)).await
			})
			.await?;
		Ok("Copied".to_string())
	}

	async fn local_to_local_copy(&self, task: Arc<ProgressHandle>, from: &Resource, to: &Resource, ctx: &ExecutionContext) -> Result<String, Error> {
		let file_size = from.backend.metadata(from.as_path()).await?.size.unwrap_or(0);

		if file_size < COPY_THRESHOLD {
			from.backend.copy(from.as_path(), to.as_path()).await?;
		} else {
			task.clone()
				.new_long_step("Copying (large file)", ctx, async {
					let stream = from.backend.download(from.as_path()).map(move |chunk: Result<Bytes, Error>| {
						if let Ok(chunk) = &chunk {
							task.increment(chunk.len() as u64);
						}
						chunk
					});
					to.backend.upload(to.as_path(), Box::pin(stream)).await
				})
				.await?;
		}
		let message = format!("Copied {} -> {}", from.as_path().shorten(5).display(), to.as_path().shorten(5).display());
		Ok(message)
	}

	pub async fn r#move(&self, from: &Arc<Resource>, to: &Arc<Resource>, ctx: Arc<ExecutionContext>) -> Result<(), Error> {
		// A native move/rename is only possible if the two resources are on the same filesystem backend.
		if &from.backend == &to.backend {
			let rename_result = from.backend.rename(from.as_path(), to.as_path()).await;

			match rename_result {
				Ok(()) => {
					// The fast, native rename succeeded. We are done.
					ctx.services
						.reporter
						.success(&format!("Moved {} -> {}", from.path.shorten(5).display(), to.path.shorten(5).display()));
					return Ok(());
				}
				Err(e) if e.is_cross_device() => {
					ctx.services.reporter.warning("Cross-device move; falling back to copy.");
				}
				Err(other_error) => {
					// Any other error (e.g., permissions, not found) is a genuine
					// failure and should be propagated immediately.
					return Err(other_error);
				}
			}
		}

		// --- Fallback Logic: Copy then Delete ---
		// If we reach this point, it's either a cross-provider operation (e.g. SFTP -> local)
		// or a failed native move that requires a fallback.

		// 1. Call the `copy` method, which will use the TaskManager to show progress.
		self.copy(from, to, &ctx).await?;

		// 2. If the copy was successful, delete the original source file.
		from.backend.delete(from.as_path()).await?;

		Ok(())
	}

	pub async fn copy(&self, from: &Arc<Resource>, to: &Arc<Resource>, ctx: &ExecutionContext) -> Result<(), Error> {
		use BackendType::*;
		let task_manager = ctx.services.task_manager.clone();

		to.backend.mk_parent(to.as_path()).await?;
		let size = from.backend.metadata(from.as_path()).await?.size;

		let title = from.path.file_name().unwrap_or_default().to_string_lossy().to_string();

		task_manager
			.with_task(&title, size, ctx, |task| async move {
				match (from.backend.kind(), to.backend.kind()) {
					(Local, Local) => self.local_to_local_copy(task, from, to, ctx).await,
					(Local, Remote) => {
						let description = format!("Uploading {}", from.as_path().display());
						self.any_to_remote_copy(task, from, to, description, ctx).await
					}
					(Remote, Remote) => {
						let description = format!("Transferring {}", from.as_path().display());
						self.any_to_remote_copy(task, from, to, description, ctx).await
					}
					(Remote, Local) => self.remote_to_local_copy(task, from, to, ctx).await,
				}
			})
			.await
	}

	pub async fn delete(&self, path: &Arc<Resource>) -> Result<(), Error> {
		let provider = &path.backend;
		provider.delete(path.as_path()).await
	}

	pub async fn mkdir(&self, path: &Arc<Resource>) -> Result<(), Error> {
		let provider = &path.backend;
		provider.mk_parent(path.as_path()).await
	}

	pub async fn hardlink(&self, from: &Arc<Resource>, to: &Arc<Resource>) -> Result<(), Error> {
		let from_provider = &from.backend;
		let to_provider = &to.backend;

		if from_provider == to_provider {
			from_provider.hardlink(from.as_path(), to.as_path()).await
		} else {
			Err(Error::ImpossibleOp("Cannot create hardlink across different filesystems".to_string()))
		}
	}

	pub async fn symlink(&self, from: &Arc<Resource>, to: &Arc<Resource>) -> Result<(), Error> {
		let from_provider = &from.backend;
		let to_provider = &to.backend;

		if from_provider == to_provider {
			from_provider.symlink(from.as_path(), to.as_path()).await
		} else {
			Err(Error::ImpossibleOp("Cannot create symlink across different filesystems".to_string()))
		}
	}
}
