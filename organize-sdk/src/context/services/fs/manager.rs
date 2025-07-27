use crate::{
	context::{
		services::{
			fs::{backup::Backup, locker::Locker, resource::Resource, undo::undo_copy::UndoCopy},
			task_manager::ProgressHandle,
		},
		settings::RunSettings,
		ExecutionContext,
	},
	engine::ConflictResolution,
	error::Error,
	location::Location,
	plugins::{
		action::Undo,
		storage::{BackendType, StorageProvider},
	},
	stdx::path::PathExt,
	templates::template::{Template, TemplateString},
};
use anyhow::{anyhow, Context, Result};
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

use super::connections::Connections;

fn default_host() -> TemplateString {
	TemplateString("local".to_string())
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
			.clone()
			.filename
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
	backends: Connections,
}

impl FileSystemManager {
	pub fn new(connections: Connections, settings: &RunSettings) -> Result<Self, Error> {
		let Connections { backends } = connections;
		let mut final_backends: HashMap<String, Arc<dyn StorageProvider + 'static>> = HashMap::new();

		for (name, provider) in backends.into_iter() {
			if settings.dry_run {
				if provider.prefix() == "vfs" {
					final_backends.insert(name, provider); // Use the user's explicitly configured DryRun provider
				} else {
					final_backends.insert(
						name.clone(),
						serde_json::from_value::<Arc<dyn StorageProvider>>(
							json!({ "type": "vfs", "simulated_host": name.clone(), "snapshot": settings.snapshot }),
						)
						.context(format!("Failed to create implicit VFS provider for connection '{name}'"))?,
					);
				}
			} else {
				final_backends.insert(name, provider);
			}
		}

		if !final_backends.contains_key("local") {
			if settings.dry_run {
				final_backends.insert(
					"local".to_string(),
					serde_json::from_value::<Arc<dyn StorageProvider>>(
						json!({ "type": "vfs", "simulated_host": "local".to_string(), "snapshot": settings.snapshot }),
					)
					.context("Failed to create default dry_run provider for 'local' backend")?,
				);
			} else {
				final_backends.insert(
					"local".to_string(),
					serde_json::from_value::<Arc<dyn StorageProvider>>(json!({ "type": "local" }))
						.context("Failed to create default 'local' file system provider")?,
				);
			}
		}

		Ok(Self {
			locker: Locker::default(),
			resources: Cache::new(10_000),
			backends: Connections { backends: final_backends },
		})
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

	pub async fn try_exists(&self, res: &Resource, _ctx: &ExecutionContext) -> Result<bool, Error> {
		res.backend.try_exists(res.as_path(), _ctx).await
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
				let mut stream = from.backend.download(from.as_path(), ctx);
				let temp_file = NamedTempFile::new()?;
				let mut writer = tokio::fs::File::create(temp_file.path()).await?;

				while let Ok(Some(chunk)) = stream.try_next().await {
					writer.write_all(&chunk).await?;
					progress.increment(chunk.len() as u64);
				}

				to.backend.rename(&temp_file.into_temp_path(), to.as_path(), ctx).await
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
				let download = from
					.backend
					.download(from.as_path(), ctx)
					.map(move |chunk: Result<Bytes, Error>| {
						if let Ok(chunk) = &chunk {
							task.increment(chunk.len() as u64);
						}
						chunk
					});
				to.backend.upload(to.as_path(), Box::pin(download), ctx).await
			})
			.await?;
		Ok(())
	}

	async fn local_to_local_copy(&self, task: Arc<ProgressHandle>, from: &Resource, to: &Resource, ctx: &ExecutionContext) -> Result<(), Error> {
		let file_size = from.backend.metadata(from.as_path(), ctx).await?.size.unwrap_or(0);

		if file_size < COPY_THRESHOLD {
			from.backend.copy(from.as_path(), to.as_path(), ctx).await?;
		} else {
			task.clone()
				.new_long_step("Copying (large file)", ctx, async {
					let stream = from
						.backend
						.download(from.as_path(), ctx)
						.map(move |chunk: Result<Bytes, Error>| {
							if let Ok(chunk) = &chunk {
								task.increment(chunk.len() as u64);
							}
							chunk
						});
					to.backend.upload(to.as_path(), Box::pin(stream), ctx).await
				})
				.await?;
		}
		Ok(())
	}

	pub async fn r#move(&self, from: &Arc<Resource>, to: &Destination, ctx: Arc<ExecutionContext>) -> Result<(), Error> {
		let to_path = to.resolve(&ctx).await?;
		let to_resource = self.get_or_init_resource(to_path, None, &to.host).await?;
		// A native move/rename is only possible if the two resources are on the same filesystem backend.
		if from.backend.as_ref() == to_resource.backend.as_ref() {
			let rename_result: Result<(), Error> = if !ctx.settings.dry_run {
				let backup = Backup::new(&ctx).await?;
				backup.persist(&ctx).await?;
				from.backend.rename(from.as_path(), to_resource.as_path(), &ctx).await?;
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

	pub async fn copy(
		&self,
		from: &Arc<Resource>,
		to: &Destination,
		ctx: &ExecutionContext,
	) -> Result<Option<(Arc<Resource>, Box<dyn Undo>)>, Error> {
		use BackendType::*;
		let task_manager = ctx.services.task_manager.clone();

		if let Some(guard) = self.locker.lock_destination(ctx, to).await? {
			let dest_resource = self.get_or_init_resource(guard.as_path().to_path_buf(), None, &to.host).await?;

			dest_resource.backend.mk_parent(dest_resource.as_path(), ctx).await?;
			let size = from.backend.metadata(from.as_path(), ctx).await?.size;

			let title = from.path.file_name().unwrap_or_default().to_string_lossy().to_string();

			let backup = Backup::new(ctx).await?;
			backup.persist(ctx).await?;

			task_manager
				.with_task(&title, size, |task| async {
					if !ctx.settings.dry_run {
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

			ctx.services.reporter.success(&format!(
				"Copied {} -> {}",
				from.as_path().shorten(5).display(),
				dest_resource.as_path().shorten(5).display()
			));
			let undo_copy = UndoCopy {
				original: from.as_path().to_path_buf(),
				new: dest_resource.as_path().to_path_buf(),
				backup,
			};
			return Ok(Some((dest_resource, Box::new(undo_copy))));
		}
		Ok(None)
	}

	pub async fn delete(&self, path: &Arc<Resource>, ctx: &ExecutionContext) -> Result<(), Error> {
		let provider = &path.backend;
		let backup = Backup::new(ctx).await?;
		backup.persist(ctx).await?;
		provider.delete(path.as_path(), ctx).await?;
		self.resources.remove(path.as_path()).await;
		ctx.services
			.reporter
			.success(&format!("Deleted {}", path.as_path().shorten(5).display(),));
		Ok(())
	}

	pub async fn mkdir(&self, path: &Arc<Resource>, ctx: &ExecutionContext) -> Result<(), Error> {
		let provider = &path.backend;
		provider.mk_parent(path.as_path(), ctx).await
	}

	pub async fn hardlink(&self, from: &Arc<Resource>, to: &Destination, ctx: &ExecutionContext) -> Result<Arc<Resource>, Error> {
		if let Some(guard) = self.locker.lock_destination(ctx, to).await? {
			let dest_resource = self.get_or_init_resource(guard.as_path().to_path_buf(), None, &to.host).await?;

			let from_provider = &from.backend;
			let to_provider = &dest_resource.backend;

			if from_provider == to_provider {
				let backup = Backup::new(ctx).await?;
				backup.persist(ctx).await?;
				from_provider.hardlink(from.as_path(), dest_resource.as_path(), ctx).await?;
			} else {
				return Err(Error::ImpossibleOp("Cannot create hardlink across different filesystems".to_string()));
			}
			self.resources
				.insert(dest_resource.as_path().to_path_buf(), dest_resource.clone())
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
		if let Some(guard) = self.locker.lock_destination(ctx, to).await? {
			let dest_resource = self.get_or_init_resource(guard.as_path().to_path_buf(), None, &to.host).await?;

			dest_resource.backend.mk_parent(dest_resource.as_path(), ctx).await?;

			let final_content = match mode {
				WriteMode::Append => {
					let existing_content = dest_resource.get_bytes(ctx).await?;
					[existing_content.as_ref(), content.as_ref()].concat().into()
				}
				WriteMode::Prepend => {
					let existing_content = dest_resource.get_bytes(ctx).await?;
					[content.as_ref(), existing_content.as_ref()].concat().into()
				}
				WriteMode::Replace => content,
			};

			dest_resource
				.backend
				.write(dest_resource.as_path(), &final_content, ctx)
				.await?;
			dest_resource
				.bytes
				.set(final_content.into())
				.expect("bytes should not be initialized");
			self.resources
				.insert(dest_resource.as_path().to_path_buf(), dest_resource.clone())
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
		if let Some(guard) = self.locker.lock_destination(ctx, to).await? {
			let dest_resource = self.get_or_init_resource(guard.as_path().to_path_buf(), None, &to.host).await?;

			let from_provider = &from.backend;
			let to_provider = &dest_resource.backend;

			if from_provider == to_provider {
				let backup = Backup::new(ctx).await?;
				backup.persist(ctx).await?;
				from_provider.symlink(from.as_path(), dest_resource.as_path(), ctx).await?;
				self.resources
					.insert(dest_resource.as_path().to_path_buf(), dest_resource.clone())
					.await;
			} else {
				return Err(Error::ImpossibleOp("Cannot create symlink across different filesystems".to_string()));
			}
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
