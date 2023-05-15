use std::path::Path;

use crate::config::filters::AsFilter;
use derive_more::Deref;
use serde::Deserialize;

#[derive(Debug, Deserialize, Deref, Clone, Eq, PartialEq)]
pub struct Extension {
	extensions: Vec<String>,
}

impl AsFilter for Extension {
	fn matches<T: AsRef<Path>>(&self, path: T) -> bool {
		match path.as_ref().extension() {
			Some(extension) => {
				dbg!(&extension);
				let extension = extension.to_str().unwrap().to_string();
				self.extensions.contains(&extension)
			}
			None => false,
		}
	}
}

#[cfg(test)]
pub mod tests {

	use std::path::PathBuf;

	use super::Extension;
	use crate::config::filters::AsFilter;

	#[test]
	fn single_match_pdf() {
		let extension = Extension {
			extensions: vec!["pdf".into()],
		};
		let path = PathBuf::from("$HOME/Downloads/test.pdf");
		assert!(extension.matches(&path))
	}
	#[test]
	fn multiple_match_pdf() {
		let extension = Extension {
			extensions: vec!["pdf".into(), "doc".into(), "docx".into()],
		};
		let path = PathBuf::from("$HOME/Downloads/test.pdf");
		assert!(extension.matches(&path))
	}

	#[test]
	fn no_match() {
		let extension = Extension {
			extensions: vec!["pdf".into(), "doc".into(), "docx".into()],
		};
		let path = PathBuf::from("$HOME/Downloads/test.jpg");
		assert!(!extension.matches(&path))
	}
}
