use std::{
	fs::Metadata,
	path::{Path, PathBuf},
	sync::Arc,
};

use anyhow::{Context as ErrorContext, Result};
use async_trait::async_trait;
use futures::{stream, StreamExt, TryStreamExt};
use organize_sdk::{
	context::ExecutionContext,
	error::Error,
	location::{
		options::{Options, Target},
		Location,
	},
	plugins::storage::StorageProvider,
	resource::Resource,
	stdx::path::{PathBufExt, PathExt},
};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug, Clone)]
pub struct LocalFileSystem;

#[async_trait]
#[typetag::serde(name = "local")]
impl StorageProvider for LocalFileSystem {
	fn prefix(&self) -> &'static str {
		"file"
	}

	async fn home(&self) -> Result<PathBuf, Error> {
		Ok(dirs::home_dir().context("unable to find home directory")?)
	}

	async fn mkdir(&self, path: &Path) -> Result<(), Error> {
		if let Some(parent) = path.parent() {
			if !tokio::fs::try_exists(parent).await.unwrap_or(false) {
				tokio::fs::create_dir_all(parent).await?;
			}
		}
		Ok(())
	}

	async fn r#move(&self, from: &Path, to: &Path) -> Result<(), Error> {
		// ctx.services.fs.ensure_parent_dir_exists(destination.as_path()).await?;
		self.mkdir(to).await?;
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

	async fn copy(&self, from: &Path, to: &Path) -> Result<(), Error> {
		self.mkdir(to).await?;

		let mut dirs = Vec::new();
		let mut files = Vec::new();
		for entry in WalkDir::new(from).into_iter().filter_map(|e| e.ok()) {
			if entry.path().is_dir() {
				dirs.push(entry.path().to_path_buf());
			} else {
				files.push(entry.path().to_path_buf());
			}
		}

		for dir in dirs {
			let relative_path = dir.strip_prefix(from).unwrap();
			let dest_path = to.join(relative_path);
			tokio::fs::create_dir_all(&dest_path).await?;
		}

		let copy_futures = files.into_iter().map(|file| {
			let relative_path = file.strip_prefix(from).unwrap().to_path_buf();
			let dest_path = to.join(relative_path);
			async move { tokio::fs::copy(file, dest_path).await.map(|_| ()).map_err(Error::from) }
		});

		stream::iter(copy_futures)
			.buffer_unordered(num_cpus::get())
			.try_collect::<()>()
			.await?;

		Ok(())
	}

	async fn delete(&self, path: &Path) -> Result<(), Error> {
		if path.is_dir() {
			tokio::fs::remove_dir_all(path).await.map_err(Error::from)
		} else {
			tokio::fs::remove_file(path).await.map_err(Error::from)
		}
	}

	async fn download(&self, from: &Path) -> Result<PathBuf, Error> {
		Ok(from.to_path_buf())
	}

	async fn download_many(&self, from: &[PathBuf]) -> Result<Vec<PathBuf>, Error> {
		Ok(from.to_vec())
	}

	async fn upload(&self, from_local: &Path, to: &Path) -> Result<(), Error> {
		self.mkdir(to).await?;
		tokio::fs::copy(from_local, to).await.map_err(Error::Io).map(|_| ())
	}

	async fn upload_many(&self, from_local: &[PathBuf], to: &[PathBuf]) -> Result<(), Error> {
		if from_local.len() != to.len() {
			return Err(Error::Other(anyhow::anyhow!(
				"Mismatched number of source and destination paths for upload_many"
			)));
		}
		for (from, to) in from_local.iter().zip(to.iter()) {
			self.mkdir(to).await?;
			tokio::fs::copy(from, to).await.map_err(Error::Io).map(|_| ())?;
		}
		Ok(())
	}

	async fn hardlink(&self, from: &Path, to: &Path) -> Result<(), Error> {
		self.mkdir(to).await?;
		tokio::fs::hard_link(from, to).await.map_err(Error::from)
	}

	async fn symlink(&self, from: &Path, to: &Path) -> Result<(), Error> {
		self.mkdir(to).await?;
		#[cfg(unix)]
		{
			tokio::fs::symlink(from, to).await.map_err(Error::from)
		}
		#[cfg(windows)]
		{
			if from.is_dir() {
				tokio::fs::symlink_dir(from, to).await.map_err(Error::from)
			} else {
				tokio::fs::symlink_file(from, to).await.map_err(Error::from)
			}
		}
	}

	async fn discover(&self, location: &Location, ctx: &ExecutionContext<'_>) -> Result<Vec<Arc<Resource>>, Error> {
		let location = Arc::new(location.clone());
		let backend = ctx.services.fs.get_provider(&location.host)?;
		let resources = WalkDir::new(&location.path)
			.min_depth(location.options.min_depth)
			.max_depth(location.options.max_depth)
			.follow_links(location.options.follow_symlinks)
			.into_iter()
			.filter_entry(|entry| self.filter_entry(entry, &location.options))
			.filter_map(|e| e.ok())
			.map(|entry| {
				let path_buf = entry.path().to_path_buf();
				path_buf.as_resource(ctx, Some(location.clone()), backend.clone())
			})
			.collect::<Vec<_>>();

		let resources = stream::iter(resources)
			.buffer_unordered(num_cpus::get())
			.collect::<Vec<_>>()
			.await;

		Ok(resources)
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

	async fn write(&self, path: &Path, content: &[u8]) -> Result<(), Error> {
		tokio::fs::write(path, content).await.map_err(Error::from)
	}
}

impl LocalFileSystem {
	fn filter_entry(&self, entry: &walkdir::DirEntry, options: &Options) -> bool {
		if options.exclude.contains(&entry.path().to_path_buf()) {
			return false;
		}
		if entry.path().is_file() && options.target == Target::Folders {
			return false;
		}
		if entry.path().is_dir() && options.target == Target::Files {
			return true;
		}

		if entry.path().is_file() {
			if let Some(extension) = entry.path().extension() {
				let partial_extensions = &["crdownload", "part", "download"];
				if partial_extensions.contains(&&*extension.to_string_lossy()) && !options.partial_files {
					return false;
				}
			}
			if entry.path().is_hidden().unwrap_or(false) && !options.hidden_files {
				return false;
			}
		}
		true
	}
}
