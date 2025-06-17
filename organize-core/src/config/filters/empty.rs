use serde::{Deserialize, Serialize};

use crate::{config::context::ExecutionContext, resource::Resource, templates::template::Template};

use super::Filter;

#[derive(Eq, PartialEq, Deserialize, Serialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Empty;

#[typetag::serde(name = "empty")]
impl Filter for Empty {
	#[tracing::instrument(ret, level = "debug")]
	fn filter(&self, src: &Resource, _: &ExecutionContext) -> bool {
		let path = &src.path();
		if path.is_file() {
			std::fs::metadata(path).map(|md| md.len() == 0).unwrap_or(false)
		} else {
			path.read_dir().map(|mut i| i.next().is_none()).unwrap_or(false)
		}
	}

	fn templates(&self) -> Vec<&Template> {
		vec![]
	}
}

#[cfg(test)]
mod tests {
	use std::io::Write;

	use tempfile::NamedTempFile;

	use crate::{
		config::{
			context::ContextHarness,
			filters::{empty::Empty, Filter},
		},
		resource::Resource,
	};

	#[test]
	fn test_file_positive() {
		let file = NamedTempFile::new().unwrap();
		let path = file.path();
		let res = Resource::new(path, path.parent().unwrap()).unwrap();
		let action = Empty;
		let harness = ContextHarness::new();
		let context = harness.context();
		assert!(action.filter(&res, &context))
	}
	#[test]
	fn test_dir_positive() {
		let dir = tempfile::tempdir().unwrap();
		let path = dir.path();
		let res = Resource::new(path, path.parent().unwrap()).unwrap();
		let action = Empty;
		let harness = ContextHarness::new();
		let context = harness.context();
		assert!(action.filter(&res, &context))
	}
	#[test]
	fn test_file_negative() {
		let mut file = NamedTempFile::new().unwrap();
		file.write_all(b"test").unwrap();
		let path = file.path();
		let res = Resource::new(path, path.parent().unwrap()).unwrap();
		let action = Empty;
		let harness = ContextHarness::new();
		let context = harness.context();
		assert!(action.filter(&res, &context))
	}
	#[test]
	fn test_dir_negative() {
		let dir = NamedTempFile::new().unwrap();
		let path = dir.path().parent().unwrap();
		let res = Resource::new(path, path.parent().unwrap()).unwrap();
		let action = Empty;
		let harness = ContextHarness::new();
		let context = harness.context();
		assert!(action.filter(&res, &context))
	}
}
