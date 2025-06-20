
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{config::context::ExecutionContext, templates::template::Template};

use super::Filter;

#[derive(Eq, PartialEq, Deserialize, Serialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Empty;

#[async_trait]
#[typetag::serde(name = "empty")]
impl Filter for Empty {
	async fn filter(&self, ctx: &ExecutionContext) -> bool {
		let path = &ctx.scope.resource.path();
		let Ok(metadata) = tokio::fs::metadata(path).await else {
			return false;
		};

		if metadata.is_file() {
			// If it's a file, check its length from the metadata we already fetched.
			metadata.len() == 0
		} else if metadata.is_dir() {
			// If it's a directory, asynchronously try to read its contents.
			let Ok(mut dir) = tokio::fs::read_dir(path).await else {
				return false; // Could not read directory, assume not empty.
			};

			// Asynchronously try to get the first entry.
			let Ok(first_entry) = dir.next_entry().await else {
				return false; // Could not read entry, assume not empty.
			};

			// If the first entry is `None`, the directory is empty.
			first_entry.is_none()
		} else {
			// The path is something else (like a symlink) that we don't consider empty.
			false
		}
	}

	fn templates(&self) -> Vec<&Template> {
		vec![]
	}
}

// #[cfg(test)]
// mod tests {
// 	use std::io::Write;

// 	use tempfile::NamedTempFile;

// 	use crate::{
// 		config::{
// 			context::ContextHarness,
// 			filters::{empty::Empty, Filter},
// 		},
// 		resource::Resource,
// 	};

// 	#[test]
// 	fn test_file_positive() {
// 		let file = NamedTempFile::new().unwrap();
// 		let path = file.path();
// 		let res = Resource::new(path, Some(path.parent().unwrap())).unwrap();
// 		let action = Empty;
// 		let harness = ContextHarness::new();
// 		let context = harness.context();
// 		assert!(action.filter(&res, &context))
// 	}
// 	#[test]
// 	fn test_dir_positive() {
// 		let dir = tempfile::tempdir().unwrap();
// 		let path = dir.path();
// 		let res = Resource::new(path, Some(path.parent().unwrap())).unwrap();
// 		let action = Empty;
// 		let harness = ContextHarness::new();
// 		let context = harness.context();
// 		assert!(action.filter(&res, &context))
// 	}
// 	#[test]
// 	fn test_file_negative() {
// 		let mut file = NamedTempFile::new().unwrap();
// 		file.write_all(b"test").unwrap();
// 		let path = file.path();
// 		let res = Resource::new(path, Some(path.parent().unwrap())).unwrap();
// 		let action = Empty;
// 		let harness = ContextHarness::new();
// 		let context = harness.context();
// 		assert!(!action.filter(&res, &context))
// 	}
// 	#[test]
// 	fn test_dir_negative() {
// 		let dir = NamedTempFile::new().unwrap();
// 		let path = dir.path().parent().unwrap();
// 		let res = Resource::new(path, Some(path.parent().unwrap())).unwrap();
// 		let action = Empty;
// 		let harness = ContextHarness::new();
// 		let context = harness.context();
// 		assert!(!action.filter(&res, &context))
// 	}
// }
