#[cfg(target_family = "windows")]
use std::path::Path;
use std::{ffi::OsStr, path::PathBuf, sync::Arc};

use async_trait::async_trait;

use crate::{context::ExecutionContext, location::Location, plugins::storage::StorageProvider, resource::Resource};

#[async_trait]
pub trait PathExt {
	type HiddenError;
	fn is_hidden(&self) -> Result<bool, Self::HiddenError>;
	async fn expand_user(self, backend: Arc<dyn StorageProvider>) -> PathBuf;
}

#[async_trait]
pub trait PathBufExt {
	async fn as_resource(self, ctx: &ExecutionContext<'_>, location: Option<Arc<Location>>, backend: Arc<dyn StorageProvider>) -> Arc<Resource>;
}

#[async_trait]
impl<T: AsRef<Path> + Sync + Send> PathExt for T {
	#[cfg(target_family = "unix")]
	type HiddenError = std::convert::Infallible;
	#[cfg(target_family = "windows")]
	type HiddenError = std::io::Error;

	async fn expand_user(self, backend: Arc<dyn StorageProvider>) -> PathBuf {
		let path = self.as_ref();
		let mut components = path.components();
		if let Some(component) = components.next() {
			if component.as_os_str() == OsStr::new("~") {
				let mut path = backend.home().await.expect("could not find home directory");
				path.extend(components);
				return path;
			}
		}
		path.to_path_buf()
	}

	#[cfg(target_family = "unix")]
	fn is_hidden(&self) -> Result<bool, Self::HiddenError> {
		match self.file_name() {
			None => Ok(false),
			Some(filename) => Ok(filename.to_string_lossy().starts_with('.')),
		}
	}

	#[cfg(target_family = "windows")]
	fn is_hidden(&self) -> Result<bool, Self::HiddenError> {
		use std::{fs, os::windows::prelude::*};
		let metadata = fs::metadata(self)?;
		let attributes = metadata.file_attributes();
		Ok((attributes & 0x2) > 0)
	}
}

#[async_trait]
impl PathBufExt for PathBuf {
	async fn as_resource(self, ctx: &ExecutionContext<'_>, location: Option<Arc<Location>>, backend: Arc<dyn StorageProvider>) -> Arc<Resource> {
		ctx.services
			.fs
			.resources
			.get_with(self.clone(), async move { Arc::new(Resource::new(self.as_ref(), location, backend)) })
			.await
	}
}
