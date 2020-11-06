use crate::user_config::rules::filters::AsFilter;
use serde::Deserialize;
use std::{borrow::Cow, path::Path};

#[derive(Eq, PartialEq, Deserialize, Debug, Clone, Default)]
pub struct Filename {
	pub startswith: Option<String>,
	pub endswith: Option<String>,
	pub contains: Option<String>,
	#[serde(default)]
	pub case_sensitive: bool,
}

impl AsFilter for Filename {
	fn matches(&self, path: &Path) -> bool {
		let mut filename = path.file_name().unwrap().to_str().unwrap().to_string();
		let mut filter = Cow::Borrowed(self);
		if !self.case_sensitive {
			filename = filename.to_lowercase();
			if let Some(startswith) = &filter.startswith {
				filter.to_mut().startswith = Some(startswith.to_lowercase())
			}
			if let Some(endswith) = &filter.endswith {
				filter.to_mut().endswith = Some(endswith.to_lowercase())
			}
			if let Some(contains) = &filter.contains {
				filter.to_mut().contains = Some(contains.to_lowercase())
			}
		}
		let mut matches = true;
		if let Some(startswith) = &filter.startswith {
			matches = matches && filename.starts_with(startswith);
		}
		if let Some(endswith) = &filter.endswith {
			matches = matches && filename.ends_with(endswith)
		}
		if let Some(contains) = &filter.contains {
			matches = matches && filename.contains(contains);
		}
		matches
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::{
		io::{Error, ErrorKind, Result},
		path::PathBuf,
	};

	#[test]
	fn match_beginning_case_insensitive() -> Result<()> {
		let path = PathBuf::from("$HOME/Downloads/test.pdf");
		let mut filename = Filename::default();
		filename.startswith = Some("TE".into());
		match filename.matches(&path) {
			true => Ok(()),
			false => Err(Error::from(ErrorKind::Other)),
		}
	}

	#[test]
	fn match_ending_case_insensitive() -> Result<()> {
		let path = PathBuf::from("$HOME/Downloads/test.pdf");
		let mut filename = Filename::default();
		filename.endswith = Some("DF".into());
		match filename.matches(&path) {
			true => Ok(()),
			false => Err(Error::from(ErrorKind::Other)),
		}
	}

	#[test]
	fn match_containing_case_insensitive() -> Result<()> {
		let path = PathBuf::from("$HOME/Downloads/test.pdf");
		let mut filename = Filename::default();
		filename.contains = Some("ES".into());
		match filename.matches(&path) {
			true => Ok(()),
			false => Err(Error::from(ErrorKind::Other)),
		}
	}

	#[test]
	fn match_beginning_case_sensitive() -> Result<()> {
		let path = PathBuf::from("$HOME/Downloads/test.pdf");
		let mut filename = Filename::default();
		filename.case_sensitive = true;
		filename.startswith = Some("TE".into());
		match filename.matches(&path) {
			false => Ok(()),
			true => Err(Error::from(ErrorKind::Other)),
		}
	}

	#[test]
	fn match_ending_case_sensitive() -> Result<()> {
		let path = PathBuf::from("$HOME/Downloads/test.pdf");
		let mut filename = Filename::default();
		filename.case_sensitive = true;
		filename.endswith = Some("DF".into());
		match filename.matches(&path) {
			false => Ok(()),
			true => Err(Error::from(ErrorKind::Other)),
		}
	}

	#[test]
	fn match_containing_case_sensitive() -> Result<()> {
		let path = PathBuf::from("$HOME/Downloads/test.pdf");
		let mut filename = Filename::default();
		filename.case_sensitive = true;
		filename.contains = Some("ES".into());
		match filename.matches(&path) {
			false => Ok(()),
			true => Err(Error::from(ErrorKind::Other)),
		}
	}
}
