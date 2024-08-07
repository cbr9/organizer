use crate::{config::filters::AsFilter, resource::Resource};
use derive_more::Deref;
use serde::Deserialize;

#[derive(Debug, Deserialize, Deref, Clone, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Extension {
	#[serde(default)]
	extensions: Vec<String>,
}

impl AsFilter for Extension {
	fn matches(&self, res: &Resource) -> bool {
		let extension = res.path.extension().unwrap_or_default().to_string_lossy().to_string();
		if extension.is_empty() {
			return false;
		}

		self.extensions.iter().any(|e| {
			let mut negate = false;
			let mut parsed = e.clone();

			if parsed.starts_with('!') {
				negate = true;
				parsed = parsed.replacen('!', "", 1);
			}
			let mut matches = parsed == extension;
			if negate {
				matches = !matches
			}
			matches
		})
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
		let path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		assert!(extension.matches(&path))
	}
	#[test]
	fn multiple_match_pdf() {
		let extension = Extension {
			extensions: vec!["pdf".into(), "doc".into(), "docx".into()],
		};
		let path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		assert!(extension.matches(&path))
	}

	#[test]
	fn no_match() {
		let extension = Extension {
			extensions: vec!["pdf".into(), "doc".into(), "docx".into()],
		};
		let path = Resource::from_str("$HOME/Downloads/test.jpg").unwrap();
		assert!(!extension.matches(&path))
	}
}
