use std::{borrow::Cow, path::PathBuf, sync::Arc};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{context::ExecutionContext, errors::Error, filter::Filter, resource::Resource, templates::template::Template};

#[derive(Eq, PartialEq, Deserialize, Serialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Empty {
	path: Option<Template>,
}

impl Empty {
	pub async fn is_empty(&self, ctx: &ExecutionContext<'_>) -> Result<bool, Error> {
		let res = ctx.scope.resource()?;
		let path_to_check = match &self.path {
			Some(path) => Cow::Owned(path.render(ctx).await.map(PathBuf::from)?),
			None => Cow::Borrowed(res.as_path()),
		};
		let metadata = ctx.scope.resource()?.location.backend.metadata(&path_to_check).await?;
		if metadata.is_file() {
			// A file is empty if its length is 0.
			Ok(metadata.len() == 0)
		} else if metadata.is_dir() {
			// A directory is empty if its `read_dir` iterator has no first entry.
			let entries = ctx.scope.resource()?.location.backend.read_dir(&path_to_check).await?;
			Ok(entries.is_empty())
		} else {
			// Symlinks, etc., are not considered empty.
			Ok(false)
		}
	}
}

#[async_trait]
#[typetag::serde(name = "empty")]
impl Filter for Empty {
	async fn filter(&self, ctx: &ExecutionContext) -> Result<Vec<Arc<Resource>>, Error> {
		if self.is_empty(ctx).await.unwrap_or(false) {
			Ok(vec![ctx.scope.resource()?])
		} else {
			Ok(vec![])
		}
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
