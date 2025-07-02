use crate::{context::ExecutionContext, error::Error, location::Location, resource::Resource};
use anyhow::Result;
use async_trait::async_trait;
use dyn_clone::DynClone;
use dyn_eq::DynEq;
use std::{
	fmt::Debug,
	fs::Metadata,
	path::{Path, PathBuf},
	sync::Arc,
};

dyn_clone::clone_trait_object!(StorageProvider);
dyn_eq::eq_trait_object!(StorageProvider);

#[async_trait]
#[typetag::serde(tag = "type")]
/// A trait for any component that can provide a list of files to be processed.
/// This could be a local folder, an S3 bucket, an SFTP connection, etc.
pub trait StorageProvider: DynEq + DynClone + Sync + Send + Debug {
	fn home(&self) -> Result<PathBuf, Error>;
	fn prefix(&self) -> &'static str;
	async fn metadata(&self, path: &Path) -> Result<Metadata, Error>;
	async fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, Error>;
	async fn read(&self, path: &Path) -> Result<Vec<u8>, Error>;
	async fn write(&self, path: &Path, content: &[u8]) -> Result<(), Error>;
	async fn discover(&self, location: &Location, ctx: &ExecutionContext<'_>) -> Result<Vec<Arc<Resource>>, Error>;
	async fn mkdir(&self, path: &Path, ctx: &ExecutionContext<'_>) -> Result<(), Error>;
	async fn r#move(&self, from: &Path, to: &Path, ctx: &ExecutionContext<'_>) -> Result<(), Error>;
	async fn copy(&self, from: &Path, to: &Path, ctx: &ExecutionContext<'_>) -> Result<(), Error>;
	async fn delete(&self, path: &Path) -> Result<(), Error>;
	async fn download(&self, from: &Path) -> Result<PathBuf, Error>;
	async fn upload(&self, from_local: &Path, to: &Path, ctx: &ExecutionContext<'_>) -> Result<(), Error>;
	async fn hardlink(&self, from: &Path, to: &Path, ctx: &ExecutionContext<'_>) -> Result<(), Error>;
	async fn symlink(&self, from: &Path, to: &Path, ctx: &ExecutionContext<'_>) -> Result<(), Error>;
}
