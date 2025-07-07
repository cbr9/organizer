use std::{
	fmt::Display,
	hash::Hash,
	path::{Path, PathBuf},
	sync::Arc,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::{fs::File, io::AsyncReadExt, sync::OnceCell};

use crate::{
	error::Error,
	location::Location,
	plugins::storage::{Metadata, StorageProvider},
};

#[derive(Debug, Default, Clone)]
pub enum FileState {
	Unknown,
	#[default]
	Exists,
	Deleted,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Resource {
	pub path: PathBuf,
	pub location: Option<Arc<Location>>,
	pub backend: Arc<dyn StorageProvider>,
	pub host: String,
	#[serde(skip)]
	mime: OnceCell<String>,
	#[serde(skip)]
	bytes: OnceCell<Vec<u8>>,
	#[serde(skip)]
	hash: OnceCell<String>,
	#[serde(skip)]
	metadata: OnceCell<Metadata>,
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
	pub fn new(path: &Path, host: String, location: Option<Arc<Location>>, backend: Arc<dyn StorageProvider>) -> Self {
		Self {
			path: path.to_path_buf(),
			host,
			location,
			backend,
			mime: OnceCell::new(),
			bytes: OnceCell::new(),
			hash: OnceCell::new(),
			metadata: OnceCell::new(),
		}
	}

	pub fn as_path(&self) -> &Path {
		self.path.as_path()
	}

	pub fn with_filename(self, filename: &str) -> Self {
		let new_path = self.path.with_file_name(filename);
		self.with_path(new_path)
	}

	pub fn with_path(self, new_path: PathBuf) -> Self {
		Self {
			path: new_path,
			location: self.location, // The origin root folder remains the same.
			backend: self.backend,
			host: self.host,

			// The content, hash, and MIME type of a file do not change when it is moved.
			// We can move these initialized OnceLock fields to the new struct to preserve the cache.
			bytes: self.bytes,
			hash: self.hash,
			mime: self.mime,

			// The filesystem metadata (like modification times of parent dirs) IS different
			// at the new location. We reset this field to force a re-fetch if needed.
			metadata: OnceCell::new(),
		}
	}

	pub async fn get_mime(&self) -> &str {
		self.mime
			.get_or_init(async || mime_guess::from_path(&self.path).first_or_octet_stream().to_string())
			.await
			.as_str()
	}

	pub async fn get_metadata(&self) -> Result<&Metadata, Error> {
		self.metadata
			.get_or_try_init(|| async { self.backend.metadata(&self.path).await })
			.await
	}

	pub async fn get_bytes(&self) -> Result<&Vec<u8>, Error> {
		self.bytes
			.get_or_try_init(|| async { self.backend.read(&self.path).await })
			.await
	}

	pub async fn get_hash(&self) -> Result<&String, Error> {
		self.hash
			.get_or_try_init(|| async {
				let mut file = File::open(&self.path).await?;
				let mut hasher = Sha256::new();
				let mut buffer = [0; 1024];
				loop {
					let count = file.read(&mut buffer).await?;
					if count == 0 {
						break;
					}
					hasher.update(&buffer[..count]);
				}
				let hash = hasher.finalize();
				let hash_str = format!("{hash:x}");
				Ok(hash_str)
			})
			.await
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
