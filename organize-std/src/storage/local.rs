use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	sync::Arc,
};

use anyhow::{Context as ErrorContext, Result};
use async_trait::async_trait;
use bytes::Bytes;
use futures::{
	future::BoxFuture,
	stream::{self, BoxStream},
	StreamExt,
};
use organize_sdk::{
	context::ExecutionContext,
	error::Error,
	location::{
		options::{Options, Target},
		Location,
	},
	plugins::storage::{BackendType, Metadata, StorageProvider},
	resource::Resource,
	stdx::path::PathExt,
};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio_util::codec::{BytesCodec, FramedRead};
use walkdir::WalkDir;

use super::IntoMetadata;

impl IntoMetadata for std::fs::Metadata {
	fn into_metadata(self) -> Metadata {
		Metadata {
			size: Some(self.len()),
			modified: self.modified().ok(),
			created: self.created().ok(),
			is_dir: self.is_dir(),
			is_file: self.is_file(),
			extra: HashMap::new(), // No extra fields for standard metadata
		}
	}
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug, Clone)]
pub struct LocalFileSystem;

#[async_trait]
#[typetag::serde(name = "local")]
impl StorageProvider for LocalFileSystem {
	fn kind(&self) -> BackendType {
		BackendType::Local
	}

	fn prefix(&self) -> &'static str {
		"file"
	}

	async fn home(&self) -> Result<PathBuf, Error> {
		Ok(dirs::home_dir().context("unable to find home directory")?)
	}

	async fn mk_parent(&self, path: &Path) -> Result<(), Error> {
		if let Some(parent) = path.parent() {
			if !tokio::fs::try_exists(parent).await.unwrap_or(false) {
				tokio::fs::create_dir_all(parent).await?;
			}
		}
		Ok(())
	}

	async fn rename(&self, from: &Path, to: &Path) -> Result<(), Error> {
		tokio::fs::rename(from, to).await.map_err(|e| Error::Io(e))
	}

	async fn copy(&self, from: &Path, to: &Path) -> Result<(), Error> {
		tokio::fs::copy(from, to).await?;

		Ok(())
	}

	async fn delete(&self, path: &Path) -> Result<(), Error> {
		if path.is_dir() {
			tokio::fs::remove_dir_all(path).await.map_err(Error::from)
		} else {
			tokio::fs::remove_file(path).await.map_err(Error::from)
		}
	}

	fn download<'a>(&'a self, path: &'a Path) -> BoxStream<'a, Result<Bytes, Error>> {
		// The `async_stream::try_stream!` macro lets us write async code
		// inside this block, and it bundles it all into a stream.
		let stream = async_stream::try_stream! {
			let file = tokio::fs::File::open(path).await?;
			let mut reader = FramedRead::new(file, BytesCodec::new());
			while let Some(chunk_result) = reader.next().await {
				// `yield` sends the next item out of the stream.
				yield chunk_result?.freeze();
			}
		};
		// We pin and box the stream to return it as a trait object.
		Box::pin(stream)
	}

	fn upload<'a>(&'a self, to: &'a Path, mut stream: BoxStream<'a, Result<Bytes, Error>>) -> BoxFuture<'a, Result<(), Error>> {
		Box::pin(async move {
			let mut file = tokio::fs::File::create(to).await?;
			while let Some(chunk_result) = stream.next().await {
				file.write_all(&chunk_result?).await?;
			}
			Ok(())
		})
	}

	async fn hardlink(&self, from: &Path, to: &Path) -> Result<(), Error> {
		self.mk_parent(to).await?;
		tokio::fs::hard_link(from, to).await.map_err(Error::from)
	}

	async fn symlink(&self, from: &Path, to: &Path) -> Result<(), Error> {
		self.mk_parent(to).await?;
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

	async fn discover(&self, location: &Location, ctx: &ExecutionContext) -> Result<Vec<Arc<Resource>>, Error> {
		let location = Arc::new(location.clone());
		let backend = ctx.services.fs.get_provider(&location.host)?;
		let resources = WalkDir::new(&location.path)
			.min_depth(location.options.min_depth)
			.max_depth(location.options.max_depth)
			.follow_links(location.options.follow_symlinks)
			.into_iter()
			.filter_map(|e| e.ok())
			.filter(|entry| self.filter_entry(entry, &location.options))
			.map(|entry| {
				ctx.services
					.fs
					.get_or_init_resource(entry.into_path(), Some(location.clone()), &location.host, backend.clone())
			})
			.collect::<Vec<_>>();

		let resources = stream::iter(resources)
			.buffer_unordered(num_cpus::get())
			.collect::<Vec<_>>()
			.await;

		Ok(resources)
	}

	async fn metadata(&self, path: &Path) -> Result<Metadata, Error> {
		Ok(tokio::fs::metadata(path).await?.into_metadata())
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
			return false;
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
