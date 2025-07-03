use crate::{
	context::{services::fs::locker::Locker, ExecutionContext},
	engine::rule::RuleBuilder,
	error::Error,
	plugins::storage::StorageProvider,
	resource::{FileState, Resource},
	templates::template::{Template, TemplateString},
};
use anyhow::Result;
use futures;
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	sync::Arc,
};


#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct DestinationBuilder {
	pub folder: TemplateString,
	pub filename: Option<TemplateString>,
	pub host: TemplateString,
}
impl DestinationBuilder {
	/// Compiles the raw DestinationBuilder into an executable Destination.
	pub fn build(self, ctx: &ExecutionContext<'_>) -> Result<Destination, Error> {
		let folder = ctx.services.compiler.compile_template(&self.folder)?;
		let filename = self.filename.map(|f| ctx.services.compiler.compile_template(&f)).transpose()?; // This elegantly handles the Option<Result<T, E>>
		let host = ctx.services.compiler.compile_template(&self.host)?;
		Ok(Destination { folder, filename, host })
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Destination {
	pub folder: Template,
	pub filename: Option<Template>,
	pub host: Template,
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

	pub async fn copy_many(&self, from: &[Arc<Resource>], to: &[Arc<Resource>]) -> Result<(), Error> {
		if from.len() != to.len() {
			return Err(Error::Other(anyhow::anyhow!(
				"Mismatched number of source and destination resources for copy_many"
			)));
		}

		let mut futures = Vec::with_capacity(from.len());
		for (from_res, to_res) in from.iter().zip(to.iter()) {
			let from_provider = from_res.backend.clone();
			let to_provider = to_res.backend.clone();
			let from_path = from_res.as_path().to_path_buf();
			let to_path = to_res.as_path().to_path_buf();
			let manager_clone = self.clone();

			futures.push(async move {
				manager_clone.ensure_parent_dir_exists(&to_path).await?;

				let from_is_local = from_provider.prefix() == "file";
				let to_is_local = to_provider.prefix() == "file";

				match (from_is_local, to_is_local) {
					(true, false) => to_provider.upload(&from_path, &to_path).await?,
					(false, true) => {
						let temp_path = from_provider.download(&from_path).await?;
						tokio::fs::copy(&temp_path, &to_path)
							.await
							.map_err(Error::Io)
							.map(|_| ())?;
						tokio::fs::remove_file(temp_path).await.map_err(Error::Io)?;
					}
					(false, false) => {
						let temp_path = from_provider.download(&from_path).await?;
						to_provider.upload(&temp_path, &to_path).await?;
					}
					(true, true) => from_provider.copy(&from_path, &to_path).await?,
				}
				Ok::<(), Error>(())
			});
		}
		futures::future::try_join_all(futures)
			.await
			.map(|_| ())
			.map_err(|e| Error::Other(anyhow::anyhow!("Failed to copy one or more resources: {}", e)))?;
		Ok(())
	}

	pub async fn copy(&self, from: &Arc<Resource>, to: &Arc<Resource>) -> Result<(), Error> {
		self.copy_many(std::slice::from_ref(from), std::slice::from_ref(to)).await
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

	pub async fn r#move(&self, from: &Arc<Resource>, to: &Arc<Resource>) -> Result<(), Error> {
		let from_provider = &from.backend;
		let to_provider = &to.backend;

		if from_provider == to_provider {
			from_provider.r#move(from.as_path(), to.as_path()).await
		} else {
			self.copy(from, to).await?;
			self.delete(from).await
		}
	}
}
