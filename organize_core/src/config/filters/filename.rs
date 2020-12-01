use std::{borrow::Cow, path::Path};

use crate::config::filters::AsFilter;
use serde::Deserialize;

#[derive(Eq, PartialEq, Deserialize, Debug, Clone, Default)]
pub struct Filename {
	pub startswith: Option<String>,
	pub endswith: Option<String>,
	pub contains: Option<String>,
	#[serde(default)]
	pub case_sensitive: bool,
}

impl AsFilter for Filename {
	fn matches<T: AsRef<Path>>(&self, path: &T) -> bool {
		let mut filename = path.as_ref().file_name().unwrap().to_str().unwrap().to_string();
		let mut filter = self.clone();
		if !self.case_sensitive {
			filename = filename.to_lowercase();
			if let Some(startswith) = &filter.startswith {
				filter.startswith = Some(startswith.to_lowercase())
			}
			if let Some(endswith) = &filter.endswith {
				filter.endswith = Some(endswith.to_lowercase())
			}
			if let Some(contains) = &filter.contains {
				filter.contains = Some(contains.to_lowercase())
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
	use std::{
		io::{Error, ErrorKind, Result},
		path::PathBuf,
	};

	use super::*;

	#[test]
	fn match_beginning_case_insensitive() -> Result<()> {
		let path = PathBuf::from("$HOME/Downloads/test.pdf");
		let filename = Filename {
			startswith: Some("TE".into()),
			..Default::default()
		};
		match filename.matches(&path) {
			true => Ok(()),
			false => Err(Error::from(ErrorKind::Other)),
		}
	}

	#[test]
	fn match_ending_case_insensitive() -> Result<()> {
		let path = PathBuf::from("$HOME/Downloads/test.pdf");
		let filename = Filename {
			endswith: Some("DF".into()),
			..Default::default()
		};
		match filename.matches(&path) {
			true => Ok(()),
			false => Err(Error::from(ErrorKind::Other)),
		}
	}

	#[test]
	fn match_containing_case_insensitive() -> Result<()> {
		let path = PathBuf::from("$HOME/Downloads/test.pdf");
		let filename = Filename {
			contains: Some("ES".into()),
			..Default::default()
		};
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
