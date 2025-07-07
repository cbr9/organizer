use crate::{
	context::{
		services::{
			fs::{
				backup::Backup,
				locker::Locker,
				resource::{FileState, Resource},
			},
			task_manager::ProgressHandle,
		},
		ExecutionContext,
	},
	engine::{rule::RuleBuilder, ConflictResolution},
	error::Error,
	location::Location,
	plugins::storage::{BackendType, StorageProvider},
	stdx::path::PathExt,
	templates::template::{Template, TemplateString},
};
use anyhow::{anyhow, Result};
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
	#[serde(default, rename = "if_exists")]
	resolution_strategy: ConflictResolution,
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
		Ok(Destination {
			folder,
			filename,
			host,
			resolution_strategy: self.resolution_strategy.clone(),
		})
	}
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum WriteMode {
	Append,
	Prepend,
	Replace,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Destination {
	pub folder: Template,
	pub filename: Option<Template>,
	pub host: String,
	pub resolution_strategy: ConflictResolution,
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
	locker: Locker,
	resources: Cache<PathBuf, Arc<Resource>>,
	// evidence of abscence > abscence of evidence
	tracked_files: Cache<PathBuf, FileState>,
	backends: HashMap<String, Arc<dyn StorageProvider>>,
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

	pub async fn get_or_init_resource(&self, path: PathBuf, location: Option<Arc<Location>>, host: &str) -> Result<Arc<Resource>> {
		let backend = self.get_provider(host)?;
		Ok(self
			.resources
			.get_with(path.clone(), async move {
				Arc::new(Resource::new(&path, host.to_string(), location, backend))
			})
			.await)
	}

	pub async fn try_exists(&self, res: &Resource, ctx: &ExecutionContext) -> Result<bool, Error> {
		if ctx.settings.dry_run {
			return match self.tracked_files.get(res.as_path()).await.unwrap_or(FileState::Unknown) {
				FileState::Exists => Ok(true),
				FileState::Deleted => Ok(false),
				FileState::Unknown => Ok(res.backend.try_exists(res.as_path()).await?),
			};
		}

		Ok(res.backend.try_exists(res.as_path()).await?)
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

	async fn remote_to_local_copy(&self, progress: Arc<ProgressHandle>, from: &Resource, to: &Resource, ctx: &ExecutionContext) -> Result<(), Error> {
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

		Ok(())
	}

	async fn any_to_remote_copy(
		&self,
		task: Arc<ProgressHandle>,
		from: &Resource,
		to: &Resource,
		description: String,
		ctx: &ExecutionContext,
	) -> Result<(), Error> {
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
		Ok(())
	}

	async fn local_to_local_copy(&self, task: Arc<ProgressHandle>, from: &Resource, to: &Resource, ctx: &ExecutionContext) -> Result<(), Error> {
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
		Ok(())
	}

	pub async fn r#move(&self, from: &Arc<Resource>, to: &Destination, ctx: Arc<ExecutionContext>) -> Result<(), Error> {
		let to_path = to.resolve(&ctx).await?;
		let to_resource = self.get_or_init_resource(to_path, None, &to.host).await?;
		// A native move/rename is only possible if the two resources are on the same filesystem backend.
		if &from.backend == &to_resource.backend {
			let rename_result: Result<(), Error> = if !ctx.settings.dry_run {
				let backup = Backup::new(&from.host, &ctx).await?;
				backup.persist(&ctx).await?;
				from.backend.rename(from.as_path(), to_resource.as_path()).await?;
				self.resources.remove(from.as_path()).await;
				self.resources
					.insert(to_resource.as_path().to_path_buf(), to_resource.clone())
					.await;
				Ok(())
			} else {
				Ok(())
			};

			match rename_result {
				Ok(()) => {
					// The fast, native rename succeeded. We are done.
					ctx.services.reporter.success(&format!(
						"Moved {} -> {}",
						from.path.shorten(5).display(),
						to_resource.path.shorten(5).display()
					));
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
		self.delete(from, &ctx).await?;

		Ok(())
	}

	pub async fn copy(&self, from: &Arc<Resource>, to: &Destination, ctx: &ExecutionContext) -> Result<Arc<Resource>, Error> {
		use BackendType::*;
		let task_manager = ctx.services.task_manager.clone();

		if let Some(guard) = self.locker.lock_destination(&ctx, to).await? {
			let dest_resource = self.get_or_init_resource(guard.as_path().to_path_buf(), None, &to.host).await?;

			dest_resource.backend.mk_parent(dest_resource.as_path()).await?;
			let size = from.backend.metadata(from.as_path()).await?.size;

			let title = from.path.file_name().unwrap_or_default().to_string_lossy().to_string();

			task_manager
				.with_task(&title, size, |task| async {
					if !ctx.settings.dry_run {
						let backup = Backup::new(&from.host, ctx).await?;
						backup.persist(ctx).await?;
						match (from.backend.kind(), dest_resource.backend.kind()) {
							(Local, Local) => self.local_to_local_copy(task, from, &dest_resource, ctx).await?,
							(Local, Remote) => {
								let description = format!("Uploading {}", from.as_path().display());
								self.any_to_remote_copy(task, from, &dest_resource, description, ctx).await?;
							}
							(Remote, Remote) => {
								let description = format!("Transferring {}", from.as_path().display());
								self.any_to_remote_copy(task, from, &dest_resource, description, ctx).await?;
							}
							(Remote, Local) => self.remote_to_local_copy(task, from, &dest_resource, ctx).await?,
						}
						self.resources
							.insert(dest_resource.as_path().to_path_buf(), dest_resource.clone())
							.await;
					}
					Ok(())
				})
				.await?;

			self.tracked_files
				.insert(dest_resource.as_path().to_path_buf(), FileState::Exists)
				.await;
			ctx.services.reporter.success(&format!(
				"Copied {} -> {}",
				from.as_path().shorten(5).display(),
				dest_resource.as_path().shorten(5).display()
			));
			Ok(dest_resource)
		} else {
			Err(Error::Other(anyhow!("could not acquire lock")))
		}
	}

	pub async fn delete(&self, path: &Arc<Resource>, ctx: &ExecutionContext) -> Result<(), Error> {
		let provider = &path.backend;
		if !ctx.settings.dry_run {
			let backup = Backup::new(&path.host, &ctx).await?;
			backup.persist(&ctx).await?;
			provider.delete(path.as_path()).await?;
			self.resources.remove(path.as_path()).await;
		}
		self.tracked_files
			.insert(path.as_path().to_path_buf(), FileState::Deleted)
			.await;
		ctx.services
			.reporter
			.success(&format!("Deleted {}", path.as_path().shorten(5).display(),));
		Ok(())
	}

	pub async fn mkdir(&self, path: &Arc<Resource>) -> Result<(), Error> {
		let provider = &path.backend;
		provider.mk_parent(path.as_path()).await
	}

	pub async fn hardlink(&self, from: &Arc<Resource>, to: &Destination, ctx: &ExecutionContext) -> Result<Arc<Resource>, Error> {
		if let Some(guard) = self.locker.lock_destination(&ctx, to).await? {
			let dest_resource = self.get_or_init_resource(guard.as_path().to_path_buf(), None, &to.host).await?;

			let from_provider = &from.backend;
			let to_provider = &dest_resource.backend;

			if from_provider == to_provider {
				if !ctx.settings.dry_run {
					let backup = Backup::new(&from.host, &ctx).await?;
					backup.persist(&ctx).await?;
					from_provider.hardlink(from.as_path(), dest_resource.as_path()).await?;
				}
			} else {
				return Err(Error::ImpossibleOp("Cannot create hardlink across different filesystems".to_string()));
			}
			self.resources
				.insert(dest_resource.as_path().to_path_buf(), dest_resource.clone())
				.await;
			self.tracked_files
				.insert(dest_resource.as_path().to_path_buf(), FileState::Exists)
				.await;
			ctx.services.reporter.success(&format!(
				"Hardlinked {} -> {}",
				from.as_path().shorten(5).display(),
				dest_resource.as_path().shorten(5).display()
			));
			Ok(dest_resource)
		} else {
			Err(Error::Other(anyhow!("could not acquire lock")))
		}
	}

	pub async fn write(&self, content: Bytes, to: &Destination, mode: WriteMode, ctx: &ExecutionContext) -> Result<Arc<Resource>, Error> {
		if let Some(guard) = self.locker.lock_destination(&ctx, to).await? {
			let dest_resource = self.get_or_init_resource(guard.as_path().to_path_buf(), None, &to.host).await?;

			dest_resource.backend.mk_parent(dest_resource.as_path()).await?;

			if !ctx.settings.dry_run {
				let final_content = match mode {
					WriteMode::Append => {
						let existing_content = dest_resource.get_bytes().await?;
						[existing_content.as_ref(), content.as_ref()].concat().into()
					}
					WriteMode::Prepend => {
						let existing_content = dest_resource.get_bytes().await?;
						[content.as_ref(), existing_content.as_ref()].concat().into()
					}
					WriteMode::Replace => content,
				};

				dest_resource.backend.write(dest_resource.as_path(), &final_content).await?;
				dest_resource
					.bytes
					.set(final_content.into())
					.expect("bytes should not be initialized");
				self.resources
					.insert(dest_resource.as_path().to_path_buf(), dest_resource.clone())
					.await;
			}

			self.tracked_files
				.insert(dest_resource.as_path().to_path_buf(), FileState::Exists)
				.await;
			ctx.services
				.reporter
				.success(&format!("Written to {}", dest_resource.as_path().shorten(5).display()));
			Ok(dest_resource)
		} else {
			Err(Error::Other(anyhow!("could not acquire lock")))
		}
	}

	pub async fn symlink(&self, from: &Arc<Resource>, to: &Destination, ctx: &ExecutionContext) -> Result<Arc<Resource>, Error> {
		if let Some(guard) = self.locker.lock_destination(&ctx, to).await? {
			let dest_resource = self.get_or_init_resource(guard.as_path().to_path_buf(), None, &to.host).await?;

			let from_provider = &from.backend;
			let to_provider = &dest_resource.backend;

			if from_provider == to_provider {
				if !ctx.settings.dry_run {
					let backup = Backup::new(&from.host, &ctx).await?;
					backup.persist(&ctx).await?;
					from_provider.symlink(from.as_path(), dest_resource.as_path()).await?;
					self.resources
						.insert(dest_resource.as_path().to_path_buf(), dest_resource.clone())
						.await;
				}
			} else {
				return Err(Error::ImpossibleOp("Cannot create symlink across different filesystems".to_string()));
			}
			self.tracked_files
				.insert(dest_resource.as_path().to_path_buf(), FileState::Exists)
				.await;
			ctx.services.reporter.success(&format!(
				"Symlinked {} -> {}",
				from.as_path().shorten(5).display(),
				dest_resource.as_path().shorten(5).display()
			));
			Ok(dest_resource)
		} else {
			Err(Error::Other(anyhow!("could not acquire lock")))
		}
	}
}
