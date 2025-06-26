use path_clean::PathClean;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
	fmt::{Debug, Display},
	fs::Metadata,
	hash::Hash,
	path::{Path, PathBuf},
	sync::OnceLock,
};
use tokio::{fs::File, io::AsyncReadExt};

use crate::{context::ExecutionContext, errors::Error};

#[derive(Debug, Default, Clone)]
pub enum FileState {
	#[default]
	Unknown,
	Exists,
	Deleted,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Resource {
	pub path: PathBuf,
	#[serde(skip)]
	pub state: FileState,
	#[serde(skip)]
	mime: OnceLock<String>,
	#[serde(skip)]
	content: OnceLock<Vec<u8>>,
	#[serde(skip)]
	hash: OnceLock<String>,
	#[serde(skip)]
	metadata: OnceLock<Metadata>,
}

impl std::ops::Deref for Resource {
	type Target = PathBuf;

	fn deref(&self) -> &Self::Target {
		&self.path
	}
}

impl AsRef<Path> for Resource {
	fn as_ref(&self) -> &Path {
		self.path.as_path()
	}
}

impl Display for Resource {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.path.display())
	}
}

impl PartialEq for Resource {
	fn eq(&self, other: &Self) -> bool {
		self.path == other.path
	}
}

impl Hash for Resource {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.path.hash(state);
	}
}

impl Eq for Resource {}

impl Resource {
	pub fn get_mime(&self) -> &str {
		match self.mime.get() {
			Some(mime) => mime.as_str(),
			None => {
				let mime = mime_guess::from_path(&self.path).first_or_octet_stream().to_string();
				self.mime.set(mime).unwrap();
				self.mime.get().unwrap().as_str()
			}
		}
	}

	pub async fn get_metadata(&self) -> &Metadata {
		match self.metadata.get() {
			Some(metadata) => metadata,
			None => {
				let metadata = tokio::fs::metadata(&self.path).await.unwrap();
				self.metadata.set(metadata).unwrap();
				self.metadata.get().unwrap()
			}
		}
	}

	pub async fn get_content(&self) -> &Vec<u8> {
		match self.content.get() {
			Some(content) => content,
			None => {
				let content = tokio::fs::read(&self.path).await.unwrap();
				self.content.set(content).unwrap();
				self.content.get().unwrap()
			}
		}
	}

	pub async fn get_hash(&self) -> &String {
		match self.hash.get() {
			Some(hash) => hash,
			None => {
				let mut file = File::open(&self.path).await.unwrap();
				let mut hasher = Sha256::new();
				let mut buffer = [0; 1024];
				loop {
					let count = file.read(&mut buffer).await.unwrap();
					if count == 0 {
						break;
					}
					hasher.update(&buffer[..count]);
				}
				let hash = hasher.finalize();
				let hash_str = format!("{:x}", hash);
				self.hash.set(hash_str).unwrap();
				self.hash.get().unwrap()
			}
		}
	}
}

impl From<&Path> for Resource {
	fn from(value: &Path) -> Self {
		Self {
			path: value.to_path_buf().clean(),
			state: FileState::default(),
			mime: OnceLock::new(),
			content: OnceLock::new(),
			hash: OnceLock::new(),
			metadata: OnceLock::new(),
		}
	}
}

impl From<PathBuf> for Resource {
	fn from(value: PathBuf) -> Self {
		Self {
			path: value.to_path_buf().clean(),
			state: FileState::default(),
			mime: OnceLock::new(),
			content: OnceLock::new(),
			hash: OnceLock::new(),
			metadata: OnceLock::new(),
		}
	}
}

// impl Serialize for Resource {
// 	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
// 	where
// 		S: Serializer,
// 	{
// 		// Serialize the PathBuf that the Arc points to.
// 		self.path.serialize(serializer)
// 	}
// }
//
impl Resource {
	pub fn new(path: &Path) -> Self {
		Self::from(path)
	}

	#[cfg(test)]
	pub fn new_tmp(filename: &str) -> Self {
		use tempfile::tempdir;
		let dir = tempdir().unwrap();
		let path = dir.path().join(filename);
		Self::from(path)
	}

	pub async fn try_exists(&self, ctx: &ExecutionContext<'_>) -> Result<bool, Error> {
		if ctx.settings.dry_run {
			return match self.state {
				FileState::Exists => Ok(true),
				FileState::Deleted => Ok(false),
				FileState::Unknown => Ok(tokio::fs::try_exists(&self.path).await?),
			};
		}

		// Otherwise, check the physical filesystem using the resource's path.
		Ok(tokio::fs::try_exists(&self.path).await?)
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
