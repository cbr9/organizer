use crate::{
	context::{services::fs::locker::Locker, ExecutionContext},
	engine::rule::RuleBuilder,
	error::Error,
	plugins::storage::StorageProvider,
	resource::{FileState, Resource},
	templates::template::{Template, TemplateString},
};
use anyhow::Result;
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	sync::Arc,
};
use url::Url; // Assuming this is needed for dry_run and context

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct DestinationBuilder {
	pub folder: TemplateString,
	pub filename: Option<TemplateString>,
}
impl DestinationBuilder {
	/// Compiles the raw DestinationBuilder into an executable Destination.
	pub fn build(self, ctx: &ExecutionContext<'_>) -> Result<Destination, Error> {
		let folder = ctx.services.compiler.compile_template(&self.folder)?;
		let filename = self.filename.map(|f| ctx.services.compiler.compile_template(&f)).transpose()?; // This elegantly handles the Option<Result<T, E>>
		Ok(Destination { folder, filename })
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Destination {
	pub folder: Template,
	pub filename: Option<Template>,
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

pub fn parse_uri(uri_str: &str) -> anyhow::Result<(String, String)> {
	// For local paths, we must construct a valid file URI first.
	if !uri_str.contains("://") {
		// let path = PathBuf::from(uri_str).clean();
		// dbg!(&path);
		// This will correctly handle paths on both Windows and Unix.
		// let url = Url::from_file_path(path).map_err(|_| anyhow::anyhow!("Invalid local path"))?;
		// Return "local" as the host (backend) and the original path.
		return Ok(("file".to_string(), uri_str.to_string()));
	}

	let url = Url::parse(uri_str)?;
	let host = url
		.host_str()
		.ok_or_else(|| anyhow::anyhow!("URI is missing a host (connection name)"))?;
	let path = url.path().to_string();

	Ok((host.to_string(), path))
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

	pub fn get_provider(&self, path: &Path) -> Result<Arc<dyn StorageProvider>> {
		let (host, _) = parse_uri(path.to_str().unwrap())?;
		self.backends
			.get(&host)
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
		let from_provider = &from.backend;
		let to_provider = &to.backend;

		let from_is_local = from_provider.prefix() == "file";
		let to_is_local = to_provider.prefix() == "file";

		match (from_is_local, to_is_local) {
			(true, false) => to_provider.upload(from.as_path(), to.as_path(), ctx).await,
			(false, true) => {
				let temp_path = from_provider.download(from.as_path()).await?;
				tokio::fs::copy(&temp_path, to.as_path())
					.await
					.map_err(|e| Error::Io(e))
					.map(|_| ())?;
				tokio::fs::remove_file(temp_path).await.map_err(|e| Error::Io(e))
			}
			(false, false) => {
				let temp_path = from_provider.download(from.as_path()).await?;
				to_provider.upload(&temp_path, to.as_path(), ctx).await
			}
			(true, true) => from_provider.copy(from.as_path(), to.as_path(), ctx).await,
		}
	}

	pub async fn delete(&self, path: &Path) -> Result<(), Error> {
		let provider = self.get_provider(path)?;
		provider.delete(path).await
	}

	pub async fn mkdir(&self, path: &Path, ctx: &ExecutionContext<'_>) -> Result<(), Error> {
		let provider = self.get_provider(path)?;
		provider.mkdir(path, ctx).await
	}

	pub async fn hardlink(&self, from: &Path, to: &Path, ctx: &ExecutionContext<'_>) -> Result<(), Error> {
		let from_provider = self.get_provider(from)?;
		let to_provider = self.get_provider(to)?;

		if from_provider == to_provider {
			from_provider.hardlink(from, to, ctx).await
		} else {
			Err(Error::ImpossibleOp("Cannot create hardlink across different filesystems".to_string()))
		}
	}

	pub async fn symlink(&self, from: &Path, to: &Path, ctx: &ExecutionContext<'_>) -> Result<(), Error> {
		let from_provider = self.get_provider(from)?;
		let to_provider = self.get_provider(to)?;

		if from_provider == to_provider {
			from_provider.symlink(from, to, ctx).await
		} else {
			Err(Error::ImpossibleOp("Cannot create symlink across different filesystems".to_string()))
		}
	}

	pub async fn r#move(&self, from: &Arc<Resource>, to: &Arc<Resource>, ctx: &ExecutionContext<'_>) -> Result<(), Error> {
		let from_provider = &from.backend;
		let to_provider = &to.backend;

		if from_provider == to_provider {
			from_provider.r#move(from.as_path(), to.as_path(), ctx).await
		} else {
			self.copy(from, to, ctx).await?;
			self.delete(from.as_path()).await
		}
	}
}
