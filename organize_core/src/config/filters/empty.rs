use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::Deserialize;

use crate::resource::Resource;

use super::AsFilter;

#[derive(Eq, PartialEq, Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Empty;

impl AsFilter for Empty {
	#[tracing::instrument(ret, level = "debug")]
	fn filter(&self, resources: &[&Resource]) -> Vec<bool> {
		resources
			.par_iter()
			.map(|res| {
				let path = &res.path;
				if path.is_file() {
					std::fs::metadata(path).map(|md| md.len() == 0).unwrap_or(false)
				} else {
					path.read_dir().map(|mut i| i.next().is_none()).unwrap_or(false)
				}
			})
			.collect()
	}
}

#[cfg(test)]
mod tests {
	use std::io::Write;

	use tempfile::NamedTempFile;

	use crate::{
		config::filters::{empty::Empty, AsFilter},
		resource::Resource,
	};

	#[test]
	fn test_file_positive() {
		let file = NamedTempFile::new().unwrap();
		let path = file.path();
		let res = Resource::from(path);
		let action = Empty;
		assert_eq!(action.filter(&[&res]), vec![true])
	}
	#[test]
	fn test_dir_positive() {
		let dir = tempfile::tempdir().unwrap();
		let path = dir.path();
		let res = Resource::from(path);
		let action = Empty;
		assert_eq!(action.filter(&[&res]), vec![true])
	}
	#[test]
	fn test_file_negative() {
		let mut file = NamedTempFile::new().unwrap();
		file.write_all(b"test").unwrap();
		let path = file.path();
		let res = Resource::from(path);
		let action = Empty;
		assert_eq!(action.filter(&[&res]), vec![false])
	}
	#[test]
	fn test_dir_negative() {
		let dir = NamedTempFile::new().unwrap();
		let path = dir.path().parent().unwrap();
		let res = Resource::from(path);
		let action = Empty;
		assert_eq!(action.filter(&[&res]), vec![false])
	}
}
