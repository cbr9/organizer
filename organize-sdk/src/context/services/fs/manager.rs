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
use anyhow::Result;
use futures::{self, future};
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	sync::Arc,
};

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

	pub async fn download_many(&self, resources: &[Arc<Resource>]) -> Result<Vec<PathBuf>, Error> {
		let mut downloaded_paths = Vec::with_capacity(resources.len());
		for resource in resources {
			let provider = &resource.backend;
			let temp_path = provider.download(resource.as_path()).await?;
			downloaded_paths.push(temp_path);
		}
		Ok(downloaded_paths)
	}

	pub async fn copy_many(&self, from: &[Arc<Resource>], to: &[Arc<Resource>], ctx: Arc<ExecutionContext<'_>>) -> Result<(), Error> {
		if from.len() != to.len() {
			return Err(Error::Other(anyhow::anyhow!(
				"Mismatched number of source and destination resources for copy_many"
			)));
		}
		if from.is_empty() {
			return Ok(()); // Nothing to do
		}

		let mut futures = Vec::with_capacity(from.len());

		for (from_res, to_res) in from.iter().zip(to.iter()) {
			// Clone Arcs for moving into the async block
			let task_manager = ctx.services.task_manager.clone();
			let from_clone = from_res.clone().clone();
			let to_clone = to_res.clone().clone();

			let source_provider = from_clone.backend.clone();
			let dest_provider = to_clone.backend.clone();

			let from_is_local = from_res.backend.prefix() == "file";
			let to_is_local = to_res.backend.prefix() == "file";
			let total_steps = if !from_is_local { 2 } else { 1 };

			// Create a future for each file copy operation.
			let future = async move {
				let title = from_clone.path.file_name().unwrap_or_default().to_string_lossy().to_string();
				// Each file gets its own `with_task` call.
				task_manager
					.with_task(&title, total_steps, |task| async move {
						match (from_is_local, to_is_local) {
							(true, true) => {
								let message = format!("Copy {} -> {}", from_clone.as_path().display(), to_clone.as_path().display());
								task.clone()
									.new_step(&message, async { dest_provider.copy(from_clone.as_path(), to_clone.as_path()).await })
									.await?;
								Ok(("Copied".to_string(), IndicatorStyle::Success))
							}
							(true, false) => {
								let message = format!(
									"Uploading {} to {} ({})",
									from_clone.as_path().display(),
									to_clone.as_path().display(),
									&to_clone.host
								);
								task.clone()
									.new_step(&message, async { dest_provider.upload(from_clone.as_path(), to_clone.as_path()).await })
									.await?;
								Ok(("Uploaded".to_string(), IndicatorStyle::Success))
							}
							(false, true) => {
								let message = format!("Downloading {}", from_clone.as_path().display());
								let temp_path = task
									.clone()
									.new_step(&message, async { source_provider.download(from_clone.as_path()).await })
									.await?;
								task.new_step("Writing to destination", async {
									let result = dest_provider.upload(&temp_path, to_clone.as_path()).await;
									let _ = tokio::fs::remove_file(temp_path).await;
									result
								})
								.await?;
								let message = format!("Downloaded {}", from_clone.as_path().display());
								Ok((message, IndicatorStyle::Success))
							}
							(false, false) => {
								let temp = task
									.clone()
									.new_step("Downloading", async { source_provider.download(from_clone.as_path()).await })
									.await?;
								task.new_step("Uploading", async {
									let result = dest_provider.upload(&temp, to_clone.as_path()).await;
									let _ = tokio::fs::remove_file(temp).await;
									result
								})
								.await?;
								Ok(("Downloaded and uploaded file to destination".to_string(), IndicatorStyle::Success))
							}
						}
					})
					.await
			};
			futures.push(future);
		}

		future::try_join_all(futures).await?;
		Ok(())
	}

	pub async fn copy(&self, from: &Arc<Resource>, to: &Arc<Resource>, ctx: Arc<ExecutionContext<'_>>) -> Result<(), Error> {
		self.copy_many(std::slice::from_ref(from), std::slice::from_ref(to), ctx).await
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
			self.copy(from, to, ctx).await?;
			self.delete(from).await
		}
	}
}
