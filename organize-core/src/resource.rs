use anyhow::bail;
use std::{
	fmt::Debug,
	hash::Hash,
	path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
	path: PathBuf,
	root: Option<PathBuf>,
}

impl Hash for Resource {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.path.hash(state)
	}
}

impl Resource {
	#[tracing::instrument(err)]
	pub fn new<T: AsRef<Path> + Debug, P: AsRef<Path> + Debug>(path: T, root: Option<P>) -> anyhow::Result<Self> {
		let path = path.as_ref().to_path_buf();
		if path.parent().is_none_or(|p| p.to_string_lossy() == "") {
			bail!(
				"Cannot create a Resource from path '{}' because it has no parent directory.",
				path.display()
			);
		}

		Ok(Self {
			path,
			root: root.map(|p| p.as_ref().to_path_buf()),
		})
	}

	pub fn with_new_path(&self, path: PathBuf) -> Self {
		Self {
			path,
			root: self.root.clone(),
		}
	}

	pub fn path(&self) -> &Path {
		self.path.as_path()
	}

	pub fn root(&self) -> Option<&Path> {
		self.root.as_deref()
	}

	pub fn set_path<T: AsRef<Path>>(&mut self, path: T) {
		self.path = path.as_ref().into();
	}

	#[cfg(test)]
	pub fn new_tmp(filename: &str) -> Self {
		use tempfile::tempdir;
		let dir = tempdir().unwrap();
		let path = dir.path().join(filename);
		Self {
			path: path.to_path_buf(),
			root: Some(dir.path().to_path_buf()),
		}
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
