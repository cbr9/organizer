use std::{borrow::Cow};

use crate::{config::filters::AsFilter, resource::Resource};
use derive_more::Deref;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::Deserialize;

#[derive(Debug, Deserialize, Deref, Clone, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Extension {
	#[serde(default)]
	pub extensions: Vec<String>,
}

impl AsFilter for Extension {
	fn filter(&self, resources: &[&Resource]) -> Vec<bool> {
		resources
			.par_iter()
			.map(|res| {
				let extension = res.path.extension().unwrap_or_default().to_string_lossy();
				if extension.is_empty() {
					return false;
				}

				if self.extensions.is_empty() {
					return true;
				}

				self.extensions.iter().any(|e| {
					let mut negate = false;
					let mut parsed = Cow::from(e);

					if parsed.starts_with('!') {
						negate = true;
						parsed = Cow::Owned(parsed.to_mut().replacen('!', "", 1));
					}

					let mut matches = parsed == extension;
					if negate {
						matches = !matches
					}
					matches
				})
			})
			.collect()
	}
}

#[cfg(test)]
pub mod tests {

	use std::str::FromStr;

	use super::Extension;
	use crate::{config::filters::AsFilter, resource::Resource};

	#[test]
	fn empty_list() {
		let extension = Extension { extensions: vec![] };
		let path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		assert_eq!(extension.filter(&[&path]), vec![true])
	}
	#[test]
	fn negative_match() {
		let extension = Extension {
			extensions: vec!["!pdf".into()],
		};
		let path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		assert_eq!(extension.filter(&[&path]), vec![false])
	}
	#[test]
	fn single_match_pdf() {
		let extension = Extension {
			extensions: vec!["pdf".into()],
		};
		let path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		assert_eq!(extension.filter(&[&path]), vec![true])
	}
	#[test]
	fn multiple_match_pdf() {
		let extension = Extension {
			extensions: vec!["pdf".into(), "doc".into(), "docx".into()],
		};
		let path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		assert_eq!(extension.filter(&[&path]), vec![true])
	}
	#[test]
	fn multiple_match_negative() {
		let extension = Extension {
			extensions: vec!["!pdf".into(), "doc".into(), "docx".into()],
		};
		let path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		assert_eq!(extension.filter(&[&path]), vec![false])
	}

	#[test]
	fn no_match() {
		let extension = Extension {
			extensions: vec!["pdf".into(), "doc".into(), "docx".into()],
		};
		let path = Resource::from_str("$HOME/Downloads/test.jpg").unwrap();
		assert_eq!(extension.filter(&[&path]), vec![false])
	}
}
