use crate::{
	context::{services::fs::resource::Resource, ExecutionContext},
	error::Error,
	location::Location,
};
use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use dyn_clone::DynClone;
use dyn_eq::DynEq;
use futures::{future::BoxFuture, stream::BoxStream};
use serde_json::Value;
use std::{
	collections::HashMap,
	fmt::Debug,
	path::{Path, PathBuf},
	sync::Arc,
	time::SystemTime,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadata {
	/// The size of the file in bytes, if available.
	pub size: Option<u64>,
	/// The last modification timestamp, if available.
	pub modified: Option<SystemTime>,
	/// The creation timestamp, if available.
	pub created: Option<SystemTime>,
	/// True if the path points to a directory.
	pub is_dir: bool,
	/// True if the path points to a file.
	pub is_file: bool,
	/// A map for any backend-specific metadata (e.g., S3 ETag, SFTP UID/GID).
	pub extra: HashMap<String, String>,
}

#[derive(PartialEq, Eq)]
pub enum BackendType {
	Local,
	Remote,
}

impl BackendType {
	/// Returns `true` if the backend type is [`Remote`].
	///
	/// [`Remote`]: BackendType::Remote
	#[must_use]
	pub fn is_remote(&self) -> bool {
		matches!(self, Self::Remote)
	}

	/// Returns `true` if the backend type is [`Local`].
	///
	/// [`Local`]: BackendType::Local
	#[must_use]
	pub fn is_local(&self) -> bool {
		matches!(self, Self::Local)
	}
}

dyn_clone::clone_trait_object!(StorageProvider);
dyn_eq::eq_trait_object!(StorageProvider);

#[async_trait]
#[typetag::serde(tag = "type")]
/// A trait for any component that can provide a list of files to be processed.
/// This could be a local folder, an S3 bucket, an SFTP connection, etc.
pub trait StorageProvider: DynEq + DynClone + Sync + Send + Debug {
	async fn home(&self) -> Result<PathBuf, Error>;
	fn prefix(&self) -> &'static str;
	fn kind(&self) -> BackendType {
		BackendType::Remote
	}
	async fn metadata(&self, path: &Path, ctx: &ExecutionContext) -> Result<Metadata, Error>;
	async fn read_dir(&self, path: &Path, ctx: &ExecutionContext) -> Result<Vec<PathBuf>, Error>;
	async fn read(&self, path: &Path, ctx: &ExecutionContext) -> Result<Vec<u8>, Error>;
	async fn write(&self, path: &Path, content: &[u8], ctx: &ExecutionContext) -> Result<(), Error>;
	async fn discover(&self, location: &Location, ctx: &ExecutionContext) -> Result<Vec<Arc<Resource>>, Error>;
	async fn mk_parent(&self, path: &Path, ctx: &ExecutionContext) -> Result<(), Error>;
	async fn rename(&self, from: &Path, to: &Path, ctx: &ExecutionContext) -> Result<(), Error>;
	async fn copy(&self, from: &Path, to: &Path, ctx: &ExecutionContext) -> Result<(), Error>;
	async fn delete(&self, path: &Path, ctx: &ExecutionContext) -> Result<(), Error>;
	fn download<'a>(&'a self, path: &'a Path, ctx: &'a ExecutionContext) -> BoxStream<'a, Result<Bytes, Error>>;
	fn upload<'a>(&'a self, to: &'a Path, stream: BoxStream<'a, Result<Bytes, Error>>, ctx: &'a ExecutionContext)
		-> BoxFuture<'a, Result<(), Error>>;
	async fn try_exists(&self, path: &Path, ctx: &ExecutionContext) -> Result<bool, Error>;
	async fn hardlink(&self, from: &Path, to: &Path, ctx: &ExecutionContext) -> Result<(), Error>;
	async fn symlink(&self, from: &Path, to: &Path, ctx: &ExecutionContext) -> Result<(), Error>;
}

pub trait StorageProviderFactory: Send + Sync {
	fn create(&self, config: Value) -> Result<Option<Arc<dyn StorageProvider>>, Error>;
}
