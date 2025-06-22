use serde::{Deserialize, Serialize};
use std::{
	fmt::Debug,
	hash::Hash,
	path::{Path, PathBuf},
	sync::Arc,
};

use serde::Serializer;

use crate::{
	config::context::{ExecutionContext, FileState},
	errors::{Error, ErrorContext},
};
#[derive(Debug, Deserialize, Hash, Clone, PartialEq, Eq)]
pub struct Resource(Arc<PathBuf>);

impl From<&Path> for Resource {
	fn from(value: &Path) -> Self {
		Self(Arc::new(value.to_path_buf()))
	}
}

impl From<PathBuf> for Resource {
	fn from(value: PathBuf) -> Self {
		Self(Arc::new(value.to_path_buf()))
	}
}

impl std::ops::Deref for Resource {
	type Target = Arc<PathBuf>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl AsRef<Path> for Resource {
	fn as_ref(&self) -> &Path {
		self.0.as_path()
	}
}

impl Serialize for Resource {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		// Serialize the PathBuf that the Arc points to.
		self.0.serialize(serializer)
	}
}
impl Resource {
	pub fn new<T: Into<PathBuf> + Debug>(path: T) -> Self {
		Self(Arc::new(path.into()))
	}

	#[cfg(test)]
	pub fn new_tmp(filename: &str) -> Self {
		use tempfile::tempdir;
		let dir = tempdir().unwrap();
		let path = dir.path().join(filename);
		Self(Arc::new(path))
	}

	pub async fn try_exists(&self, ctx: &ExecutionContext<'_>) -> Result<bool, Error> {
		if ctx.settings.dry_run {
			if let Some(entry) = ctx.services.blackboard.known_paths.get(self) {
				return match entry.value() {
					FileState::Exists => Ok(true),
					FileState::Deleted => Ok(false),
				};
			}
		}

		// Otherwise, check the physical filesystem using the resource's path.
		tokio::fs::try_exists(self.as_path()).await.map_err(|e| Error::Io {
			source: e,
			path: self.clone(),
			target: None,
			context: ErrorContext::from_scope(&ctx.scope),
		})
	}
}

// #[cfg(test)]
// mod tests {
// 	use super::*;
// 	use std::path::PathBuf;

// 	#[test]
// 	fn new_with_valid_path_succeeds() {
// 		let path = PathBuf::from("/tmp/test.txt");
// 		let root = PathBuf::from("/tmp");
// 		let resource = Resource::new(&path, &root).unwrap();
// 		assert_eq!(resource.path(), &path);
// 		assert_eq!(resource.root(), &root);
// 	}

// 	#[test]
// 	fn new_with_root_path_returns_err() {
// 		let path = PathBuf::from("/");
// 		let result = Resource::new(&path, &path);
// 		assert!(result.is_err());
// 	}

// 	#[test]
// 	fn new_with_dot_path_succeeds_on_windows_fails_on_unix() {
// 		let path = PathBuf::from(".");
// 		let result = Resource::new(&path, &path);
// 		assert!(result.is_err());
// 	}

// 	#[test]
// 	fn new_with_relative_path_succeeds() {
// 		let path = PathBuf::from("some/dir/file.txt");
// 		let result = Resource::new(&path, "some/dir");
// 		assert!(result.is_ok());
// 	}

// 	#[test]
// 	fn new_with_bare_filename_returns_err() {
// 		// A bare filename like "file.txt" has an empty parent, which the new logic correctly rejects.
// 		let path = PathBuf::from("file.txt");
// 		let result = Resource::new(&path, ".");
// 		assert!(result.is_err());
// 	}
// }
