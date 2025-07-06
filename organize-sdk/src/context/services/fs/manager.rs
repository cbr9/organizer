use crate::{
	context::{
		services::{fs::locker::Locker, reporter::ui::IndicatorStyle},
		ExecutionContext,
	},
	engine::rule::RuleBuilder,
	error::Error,
	plugins::storage::StorageProvider,
	resource::{FileState, Resource},
	templates::template::{Template, TemplateString},
};
use anyhow::{Context, Result};
use futures::{self, future, TryStreamExt};
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

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct DestinationBuilder {
	pub folder: TemplateString,
	pub filename: Option<TemplateString>,
	#[serde(default = "default_host")]
	pub host: TemplateString,
}
impl DestinationBuilder {
	/// Compiles the raw DestinationBuilder into an executable Destination.
	pub async fn build(&self, ctx: &ExecutionContext<'_>) -> Result<Destination, Error> {
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
	pub async fn resolve(&self, ctx: &ExecutionContext<'_>) -> Result<PathBuf> {
		let mut folder = PathBuf::from(self.folder.render(ctx).await?);
		if let Some(filename_template) = &self.filename {
			let filename = filename_template.render(ctx).await?;
			folder.push(filename);
		}
		// A placeholder for the original filename if `filename` is not provided.
		// This would need access to the resource from the context.
		else if let Ok(resource) = ctx.scope.resource() {
			if let Some(name) = resource.path.file_name() {
				folder.push(name);
			}
		}
		Ok(folder)
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

	pub async fn copy(&self, from: &Arc<Resource>, to: &Arc<Resource>, ctx: &ExecutionContext<'_>) -> Result<(), Error> {
		let task_manager = ctx.services.task_manager.clone();

		let source_provider = from.backend.clone();
		let dest_provider = to.backend.clone();

		let from_is_remote = from.backend.prefix() != "file";
		let to_is_local = to.backend.prefix() == "file";
		let total_steps = if from_is_remote { 2 } else { 1 };
		let size = from.backend.metadata(from.as_path()).await?.size;

		let title = from.path.file_name().unwrap_or_default().to_string_lossy().to_string();
		// Each file gets its own `with_task` call.
		task_manager
			.with_task(&title, total_steps, size, |task| async move {
				match (from_is_remote, to_is_local) {
					// Case: Remote -> Local (or Remote -> Remote)
					(true, _) => {
						let temp_path = task
							.clone()
							.new_step::<PathBuf, Error, _>(&format!("Downloading {}", from.as_path().display()), async {
								let mut stream = from.backend.download(from.as_path());
								let temp_file = NamedTempFile::new()?;
								let mut writer = tokio::fs::File::create(temp_file.path()).await?;

								// Consume the stream, updating the bar after each chunk
								while let Ok(Some(chunk)) = stream.try_next().await {
									writer.write(&chunk).await?;
									task.increment_progress_bar(chunk.len() as u64);
								}

								Ok(temp_file.keep().context("could not persist temporary file")?.1) // Return the path to the temp file
							})
							.await?;

						task.new_step::<(), Error, _>("Writing to destination", async {
							if to.backend.prefix() == "file" {
								to.backend.r#move(&temp_path, to.as_path()).await?;
							} else {
								to.backend.upload(&temp_path, to.as_path()).await?;
								tokio::fs::remove_file(temp_path).await?;
							}
							Ok(())
						})
						.await?;

						Ok((
							format!("Transferred {} -> {}", from.as_path().display(), to.as_path().display()),
							IndicatorStyle::Success,
						))
					}
					// Case: Local -> Remote
					(false, true) => {
						task.new_step("Uploading", async { to.backend.upload(from.as_path(), to.as_path()).await })
							.await?;
						Ok(("✓ Uploaded".to_string(), IndicatorStyle::Success))
					}
					// Case: Local -> Local
					(false, false) => {
						task.new_step("Copying", async { from.backend.copy(from.as_path(), to.as_path()).await })
							.await?;
						Ok(("Copied".to_string(), IndicatorStyle::Success))
					}
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
		provider.mkdir(path.as_path()).await
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

	pub async fn r#move(&self, from: &Arc<Resource>, to: &Arc<Resource>, ctx: Arc<ExecutionContext<'_>>) -> Result<(), Error> {
		let from_provider = &from.backend;
		let to_provider = &to.backend;

		if from_provider == to_provider {
			from_provider.r#move(from.as_path(), to.as_path()).await
		} else {
			self.copy(from, to, &ctx).await?;
			self.delete(from).await
		}
	}
}
