use crate::{
	context::{services::fs::locker::Locker, ExecutionContext},
	errors::Error,
	folder::LocalFileSystem,
	resource::{FileState, Resource},
	storage::StorageProvider,
	templates::template::Template,
};
use anyhow::Result;
use moka::future::Cache;
use path_clean::PathClean;
use serde::{Deserialize, Serialize};
use std::{
	collections::HashMap,
	iter::FromIterator,
	path::{Path, PathBuf},
	sync::Arc,
};
use url::Url; // Assuming this is needed for dry_run and context

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Destination {
	pub folder: Template,
	pub filename: Option<Template>,
}

impl Destination {
	pub async fn resolve(&self, ctx: &ExecutionContext<'_>) -> Result<PathBuf, Error> {
		let folder = self.folder.render(ctx).await?;

		let mut folder = PathBuf::from(folder).clean();
		let filename = if let Some(filename) = &self.filename {
			filename.render(ctx).await?
		} else {
			ctx.scope
				.resource()?
				.as_path()
				.file_name()
				.unwrap()
				.to_string_lossy()
				.to_string()
		};

		let filename = PathBuf::from(filename).clean();
		folder.push(filename);
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
		return Ok(("local".to_string(), uri_str.to_string()));
	}

	let url = Url::parse(uri_str)?;
	let host = url
		.host_str()
		.ok_or_else(|| anyhow::anyhow!("URI is missing a host (connection name)"))?;
	let path = url.path().to_string();

	Ok((host.to_string(), path))
}

impl Default for FileSystemManager {
	fn default() -> Self {
		Self::new()
	}
}

impl FileSystemManager {
	pub fn new() -> Self {
		let local: Arc<dyn StorageProvider> = Arc::new(LocalFileSystem);
		let backends = HashMap::from_iter(vec![("local".to_string(), local)]);
		Self {
			locker: Locker::default(),
			resources: Cache::new(10_000),
			tracked_files: Cache::new(10_000),
			backends,
		}
	}

	pub async fn ensure_parent_dir_exists(&self, path: &Path) -> std::io::Result<()> {
		if let Some(parent) = path.parent() {
			if !tokio::fs::try_exists(parent).await.unwrap_or(false) {
				tokio::fs::create_dir_all(parent).await?;
			}
		}
		// resources: CacheBuilder::new(1_000_000)
		// 	.time_to_live(Duration::new(60 * 60 * 24, 0)) // ONE DAY
		// 	.name("cached_resources")
		// 	.build(),
		Ok(())
	}

	pub async fn r#move(&self, source: Arc<Resource>, destination: Arc<Resource>) -> Result<(), Error> {
		// Attempt a direct rename first
		self.ensure_parent_dir_exists(destination.as_path()).await?;
		match tokio::fs::rename(source.as_path(), destination.as_path()).await {
			Ok(_) => Ok(()),
			Err(e) if e.raw_os_error() == Some(libc::EXDEV) || e.kind() == std::io::ErrorKind::CrossesDevices => {
				// Handle "Cross-device link" error (EXDEV on Unix, specific error kind on Windows)
				// This means source and destination are on different file systems.
				tracing::warn!(
					"Attempting copy-then-delete for move operation due to cross-device link: {} to {}",
					source.as_path().display(),
					destination.as_path().display()
				);

				// Perform copy
				tokio::fs::copy(source.as_path(), destination.as_path()).await?;

				// If copy is successful, delete the original
				Ok(tokio::fs::remove_file(source.as_path()).await?)
			}
			Err(e) => Err(Error::Io(e)),
		}
	}
}
