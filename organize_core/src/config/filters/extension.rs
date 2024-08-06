use crate::{config::filters::AsFilter, resource::Resource};
use derive_more::Deref;
use serde::Deserialize;

#[derive(Debug, Deserialize, Deref, Clone, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Extension {
	extensions: Vec<String>,
}

impl AsFilter for Extension {
	fn matches(&self, res: &Resource) -> bool {
		let path = res.path();
		if path.is_file() {
			return path
				.extension()
				.and_then(|ext| ext.to_str())
				.map(|s| self.extensions.contains(&s.to_string()))
				.unwrap_or(false);
		}
		true
	}
}

#[cfg(test)]
pub mod tests {

	use std::str::FromStr;

	use super::Extension;
	use crate::{config::filters::AsFilter, resource::Resource};

	#[test]
	fn single_match_pdf() {
		let extension = Extension {
			extensions: vec!["pdf".into()],
		};
		let mut path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		assert!(extension.matches(&mut path))
	}
	#[test]
	fn multiple_match_pdf() {
		let extension = Extension {
			extensions: vec!["pdf".into(), "doc".into(), "docx".into()],
		};
		let mut path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		assert!(extension.matches(&mut path))
	}

	#[test]
	fn no_match() {
		let extension = Extension {
			extensions: vec!["pdf".into(), "doc".into(), "docx".into()],
		};
		let mut path = Resource::from_str("$HOME/Downloads/test.jpg").unwrap();
		assert!(!extension.matches(&mut path))
	}
}
