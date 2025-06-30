#[cfg(target_family = "windows")]
use std::path::Path;
use std::{ffi::OsStr, path::PathBuf, sync::Arc};

use async_trait::async_trait;

use crate::{context::ExecutionContext, folder::Location, resource::Resource};

#[async_trait]
pub trait PathExt {
	type HiddenError;
	fn is_hidden(&self) -> Result<bool, Self::HiddenError>;
	fn expand_user(self) -> PathBuf;
	async fn as_resource(&self, ctx: &ExecutionContext, location: Arc<Location>) -> Arc<Resource>;
}

#[async_trait]
impl<T: AsRef<Path> + Sync + Send> PathExt for T {
	#[cfg(target_family = "unix")]
	type HiddenError = std::convert::Infallible;
	#[cfg(target_family = "windows")]
	type HiddenError = std::io::Error;

	fn expand_user(self) -> PathBuf {
		let path = self.as_ref();
		let mut components = path.components();
		if let Some(component) = components.next() {
			if component.as_os_str() == OsStr::new("~") {
				let mut path = dirs::home_dir().expect("could not find home directory");
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

	async fn as_resource(&self, ctx: &ExecutionContext, location: Arc<Location>) -> Arc<Resource> {
		ctx.services
			.fs
			.resources
			.get_with(self.as_ref().to_path_buf(), async move {
				Arc::new(Resource::new(&self.as_ref().to_path_buf(), location))
			})
			.await
	}
}

#[cfg(test)]
mod tests {

	use super::*;

	#[cfg(target_family = "unix")]
	#[test]
	fn check_hidden() {
		use super::*;
		let path = Path::new("/home/user/.testfile");
		assert!(path.is_hidden().unwrap())
	}

	#[cfg(target_family = "windows")]
	#[test]
	fn not_hidden() {
		use tempfile::NamedTempFile;

		use super::*;
		let file = NamedTempFile::new().unwrap();
		let path = file.path();
		assert!(!path.is_hidden().unwrap());
	}

	#[test]
	#[cfg(target_family = "windows")]
	fn check_hidden() {
		use super::*;
		use tempfile::NamedTempFile;

		let file = NamedTempFile::new().unwrap();
		let path = file.path();
		// Use the `attrib` command on Windows to set the hidden attribute.
		let status = std::process::Command::new("attrib")
			.arg("+h")
			.arg(path.as_os_str())
			.status()
			.expect("failed to execute attrib command");
		assert!(status.success(), "attrib command failed");
		assert!(path.is_hidden().unwrap());
	}

	#[test]
	fn invalid_tilde() {
		let original = dirs::home_dir().unwrap().join("Documents~");
		assert_eq!(original.clone().expand_user(), original)
	}

	#[test]
	fn user_tilde() {
		let original = "~/Documents";
		let expected = dirs::home_dir().unwrap().join("Documents");
		assert_eq!(original.expand_user(), expected)
	}
}
