mod de;

use std::{ops::Deref, path::Path};

use crate::config::filters::AsFilter;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Extension(Vec<String>);

impl Deref for Extension {
	type Target = Vec<String>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl AsFilter for Extension {
	fn matches<T: AsRef<Path>>(&self, path: &T) -> bool {
		match path.as_ref().extension() {
			Some(extension) => {
				let extension = extension.to_str().unwrap().to_string();
				self.contains(&extension)
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
		let extension = Extension(vec!["pdf".into()]);
		let path = PathBuf::from("$HOME/Downloads/test.pdf");
		assert!(extension.matches(&path))
	}
	#[test]
	fn multiple_match_pdf() {
		let extension = Extension(vec!["pdf".into(), "doc".into(), "docx".into()]);
		let path = PathBuf::from("$HOME/Downloads/test.pdf");
		assert!(extension.matches(&path))
	}

	#[test]
	fn no_match() {
		let extension = Extension(vec!["pdf".into(), "doc".into(), "docx".into()]);
		let path = PathBuf::from("$HOME/Downloads/test.jpg");
		assert!(!extension.matches(&path))
	}
}
