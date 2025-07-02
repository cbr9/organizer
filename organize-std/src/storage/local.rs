use std::{
	collections::HashSet,
	fs::Metadata,
	path::{Path, PathBuf},
	sync::Arc,
};

use anyhow::{Context as ErrorContext, Result};
use async_trait::async_trait;
use futures::{stream, StreamExt};
use organize_sdk::{
	context::ExecutionContext,
	error::Error,
	location::{
		options::{Options, Target},
		Location,
	},
	plugins::storage::StorageProvider,
	resource::Resource,
	stdx::path::PathExt,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug, Clone)]
pub struct LocalFileSystem;

#[async_trait]
#[typetag::serde(name = "local")]
impl StorageProvider for LocalFileSystem {
	fn prefix(&self) -> &'static str {
		"file"
	}

	fn home(&self) -> Result<PathBuf, Error> {
		Ok(dirs::home_dir().context("unable to find home directory")?)
	}

	async fn mkdir(&self, path: &Path, ctx: ExecutionContext<'_>) -> Result<(), Error> {
		if let Some(parent) = path.parent() {
			if !tokio::fs::try_exists(parent).await.unwrap_or(false) {
				tokio::fs::create_dir_all(parent).await?;
			}
		}
		Ok(())
	}

	async fn r#move(&self, from: &Path, to: &Path, ctx: ExecutionContext<'_>) -> Result<(), Error> {
		// ctx.services.fs.ensure_parent_dir_exists(destination.as_path()).await?;
		self.mkdir(to, ctx).await?;
		match tokio::fs::rename(from, to).await {
			Ok(_) => Ok(()),
			Err(e) if e.raw_os_error() == Some(libc::EXDEV) || e.kind() == std::io::ErrorKind::CrossesDevices => {
				// Handle "Cross-device link" error (EXDEV on Unix, specific error kind on Windows)
				// This means source and destination are on different file systems.
				tracing::warn!(
					"Attempting copy-then-delete for move operation due to cross-device link: {} to {}",
					from.display(),
					to.display()
				);

				tokio::fs::copy(from, to).await?;
				Ok(tokio::fs::remove_file(from).await?)
			}
			Err(e) => Err(Error::Io(e)),
		}
	}

	async fn copy(&self, from: &Path, to: &Path, ctx: ExecutionContext<'_>) -> Result<(), Error> {
		todo!()
	}

	async fn delete(&self, path: &Path) -> Result<(), Error> {
		todo!()
	}

	async fn download(&self, from: &Path) -> Result<PathBuf, Error> {
		Ok(PathBuf::new())
	}

	async fn upload(&self, from_local: &Path, to: &Path, ctx: ExecutionContext<'_>) -> Result<(), Error> {
		Ok(())
	}

	async fn hardlink(&self, from: &Path, to: &Path, ctx: ExecutionContext<'_>) -> Result<(), Error> {
		todo!()
	}

	async fn symlink(&self, from: &Path, to: &Path, ctx: ExecutionContext<'_>) -> Result<(), Error> {
		todo!()
	}

	async fn discover(&self, location: &Location, ctx: &ExecutionContext<'_>) -> Result<Vec<Arc<Resource>>, Error> {
		let concurrency_limit = 50;
		let home = self.home()?;
		let min_depth = {
			let base = if location.path == home {
				1.0 as usize
			} else {
				location.options.min_depth
			};
			(base as f64).max(1.0) as usize
		};

		let max_depth = if location.path == home {
			1.0 as usize
		} else {
			location.options.max_depth
		};

		let mut collected_paths = Vec::new();
		let mut dirs_to_visit: Vec<(PathBuf, usize)> = vec![];

		let excluded_paths_set: HashSet<&PathBuf> = location.options.exclude.iter().collect();

		if excluded_paths_set.contains(&location.path) {
			tracing::warn!(
				"Start directory '{}' is in the excluded paths. Aborting search.",
				&location.path.display()
			);
			return Ok(Vec::new());
		}

		if min_depth == 0 {
			collected_paths.push(location.path.clone());
		}

		dirs_to_visit.push((location.path.clone(), 0));

		while let Some((current_dir, current_depth)) = dirs_to_visit.pop() {
			if current_depth >= max_depth {
				continue;
			}

			let mut entries = match tokio::fs::read_dir(&current_dir).await {
				Ok(e) => e,
				Err(e) => {
					eprintln!("Warning: Could not read directory {}: {}", current_dir.display(), e);
					continue;
				}
			};

			while let Some(entry) = entries.next_entry().await? {
				let path = entry.path();
				let next_depth = current_depth + 1;

				// --- Exclusion Logic for encountered paths (files or directories) ---
				if excluded_paths_set.contains(&path) {
					eprintln!("Excluding path: {}", path.display());
					// If it's a directory, we effectively prune the branch.
					// If it's a file, we just don't collect it.
					continue;
				}
				// --- End Exclusion Logic ---

				// Only add &location.path if it's within the specified depth range
				if next_depth >= min_depth && next_depth <= max_depth {
					collected_paths.push(path.clone());
				}

				// If it's a directory and still within max_depth, add it to dirs_to_visit
				// (after checking for exclusion, which is done above)
				if path.is_dir() && next_depth < max_depth {
					dirs_to_visit.push((path, next_depth));
				}
			}
		}

		// let all_files = self.find_all_files(min_depth, max_depth, ctx).await?;
		let location: Arc<Location> = Arc::new(location.clone());
		let resource_creation_futures = collected_paths
			.into_iter()
			.filter(|e| self.filter_entries(e, &location.options))
			.map(|e| {
				// Capture `e` by moving it into the async block
				// Capture `ctx` by reference (or clone/Arc if its lifetime is an issue)
				let ctx_ref = ctx;
				let location = location.clone();
				async move {
					e.as_resource(ctx_ref, location).await // Returns Result<Resource, AsResourceError>
				}
			});

		Ok(stream::iter(resource_creation_futures) // stream::iter expects an Iterator<Item=Future>
			.buffer_unordered(concurrency_limit) // Execute Futures concurrently
			.collect()
			.await)
	}

	async fn metadata(&self, path: &Path) -> Result<Metadata, Error> {
		Ok(tokio::fs::metadata(path).await?)
	}

	async fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, Error> {
		let mut dir = tokio::fs::read_dir(path).await?;
		let mut paths = vec![];
		while let Some(entry) = dir.next_entry().await? {
			paths.push(entry.path());
		}
		Ok(paths)
	}

	async fn read(&self, path: &Path) -> Result<Vec<u8>, Error> {
		Ok(tokio::fs::read(path).await?)
	}

	async fn write(&self, _path: &Path, _content: &[u8]) -> Result<()> {
		todo!()
	}
}

impl LocalFileSystem {
	fn filter_entries(&self, path: &Path, options: &Options) -> bool {
		if path.is_file() && options.target == Target::Folders {
			return false;
		}
		if path.is_dir() && options.target == Target::Files {
			return false;
		}

		if path.is_file() {
			if let Some(extension) = path.extension() {
				let partial_extensions = &["crdownload", "part", "download"];
				if partial_extensions.contains(&&*extension.to_string_lossy()) && !options.partial_files {
					return false;
				}
			}
			if path.is_hidden().unwrap_or(false) && !options.hidden_files {
				return false;
			}
		}
		true
	}
}
