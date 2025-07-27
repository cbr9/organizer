use std::path::Path;
use std::{
	ffi::OsStr,
	path::{Component, PathBuf},
	sync::Arc,
};

use async_trait::async_trait;
use path_clean::PathClean;

use crate::plugins::storage::StorageProvider;

#[async_trait]
pub trait PathExt {
	type HiddenError;
	fn is_hidden(&self) -> Result<bool, Self::HiddenError>;
	async fn expand_user(self, backend: Arc<dyn StorageProvider>) -> PathBuf;
	fn shorten(&self, max_depth: usize) -> PathBuf;
	fn normalize(&self) -> PathBuf;
}

#[async_trait]
impl<T: AsRef<Path> + Sync + Send> PathExt for T {
	#[cfg(target_family = "unix")]
	type HiddenError = std::convert::Infallible;
	#[cfg(target_family = "windows")]
	type HiddenError = std::io::Error;

	fn normalize(&self) -> PathBuf {
		let path = self.as_ref().clean();
		let mut normalized_str = path.to_string_lossy().into_owned();

		// 1. Replace all backslashes with forward slashes (for Windows compatibility)
		normalized_str = normalized_str.replace("\\", "/");

		// 2. Handle Windows drive letters (e.g., "C:/foo" -> "/foo")
		// If it starts with a drive letter (like 'C') followed by a colon ':', strip it.
		#[cfg(windows)]
		{
			if let Some(pos) = normalized_str.find(':') {
				if pos == 1 && (normalized_str.as_bytes()[0] as char).is_ascii_alphabetic() {
					// Take the substring after the colon (e.g., from "C:/path" to "/path")
					normalized_str = normalized_str[pos + 1..].to_string();
				}
			}
		}

		// 4. Use path-clean to simplify and canonicalize components (e.g., "/a/./b" -> "/a/b", "/a/b/../c" -> "/a/c")
		PathBuf::from(normalized_str)
	}

	fn shorten(&self, max_depth: usize) -> PathBuf {
		let path = self.as_ref();
		let components: Vec<Component> = path.components().collect();
		if components.len() <= max_depth {
			return path.to_path_buf();
		}

		if max_depth < 3 {
			return path.to_path_buf();
		}

		let mut result = PathBuf::new();
		let num_to_take_start = (max_depth - 1) / 2;
		let num_to_take_end = max_depth - num_to_take_start - 1;

		for component in components.iter().take(num_to_take_start) {
			result.push(component.as_os_str());
		}

		result.push("...");

		for component in components.iter().rev().take(num_to_take_end).rev() {
			result.push(component.as_os_str());
		}
		result
	}

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
		match self.as_ref().file_name() {
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
