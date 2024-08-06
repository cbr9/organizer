use crate::{config::filters::AsFilter, resource::Resource};
use serde::Deserialize;

// TODO: refactor

#[derive(Eq, PartialEq, Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Filename {
	pub startswith: Option<String>,
	pub endswith: Option<String>,
	pub contains: Option<String>,
	#[serde(default)]
	pub case_sensitive: bool,
}

impl AsFilter for Filename {
	fn matches(&self, res: &Resource) -> bool {
		let path = &res.path;
		let mut filename = path.file_name().unwrap().to_str().unwrap().to_string();
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
	use std::str::FromStr;

	use super::*;

	#[test]
	fn match_beginning_case_insensitive() {
		let mut path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		let filename = Filename {
			startswith: Some("TE".into()),
			..Default::default()
		};
		assert!(filename.matches(&mut path))
	}

	#[test]
	fn match_ending_case_insensitive() {
		let mut path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		let filename = Filename {
			endswith: Some("DF".into()),
			..Default::default()
		};
		assert!(filename.matches(&mut path))
	}

	#[test]
	fn match_containing_case_insensitive() {
		let mut path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		let filename = Filename {
			contains: Some("ES".into()),
			..Default::default()
		};
		assert!(filename.matches(&mut path))
	}

	#[test]
	fn no_match_beginning_case_sensitive() {
		let mut path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		let filename = Filename {
			case_sensitive: true,
			startswith: Some("TE".into()),
			..Default::default()
		};
		assert!(!filename.matches(&mut path))
	}

	#[test]
	fn no_match_ending_case_sensitive() {
		let mut path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		let filename = Filename {
			case_sensitive: true,
			startswith: Some("DF".into()),
			..Default::default()
		};
		assert!(!filename.matches(&mut path))
	}

	#[test]
	fn no_match_containing_case_sensitive() {
		let mut path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		let filename = Filename {
			case_sensitive: true,
			contains: Some("ES".into()),
			..Default::default()
		};
		assert!(!filename.matches(&mut path))
	}
	#[test]
	fn match_containing_case_sensitive() {
		let mut path = Resource::from_str("$HOME/Downloads/tESt.pdf").unwrap();
		let filename = Filename {
			case_sensitive: true,
			contains: Some("ES".into()),
			..Default::default()
		};
		assert!(filename.matches(&mut path))
	}
}
