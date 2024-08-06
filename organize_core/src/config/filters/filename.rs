use crate::{config::filters::AsFilter, resource::Resource, templates::TERA};
use serde::Deserialize;

// TODO: refactor

#[derive(Eq, PartialEq, Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Filename {
	#[serde(default)]
	pub startswith: Vec<String>,
	#[serde(default)]
	pub endswith: Vec<String>,
	#[serde(default)]
	pub contains: Vec<String>,
	#[serde(default)]
	pub case_sensitive: bool,
}

impl AsFilter for Filename {
	fn matches(&self, res: &Resource) -> bool {
		let filename = res.path.file_name().unwrap_or_default().to_string_lossy().to_string();
		if filename.is_empty() {
			return false;
		}

		let startswith = self
			.startswith
			.iter()
			.map(|s| TERA.lock().unwrap().render_str(s, &res.context).unwrap())
			.map(|s| if !self.case_sensitive { s.to_lowercase() } else { s })
			.any(|mut s| {
				let mut negate = false;

				if s.starts_with('!') {
					negate = true;
					s = s.replacen('!', "", 1);
				}
				let mut matches = filename.starts_with(&s);
				if negate {
					matches = !matches
				}
				matches
			});

		let endswith = self
			.endswith
			.iter()
			.map(|s| TERA.lock().unwrap().render_str(s, &res.context).unwrap())
			.map(|s| if !self.case_sensitive { s.to_lowercase() } else { s })
			.any(|mut s| {
				let mut negate = false;

				if s.starts_with('!') {
					negate = true;
					s = s.replacen('!', "", 1);
				}
				let mut matches = filename.ends_with(&s);
				if negate {
					matches = !matches
				}
				matches
			});

		let contains = self
			.contains
			.iter()
			.map(|s| TERA.lock().unwrap().render_str(s, &res.context).unwrap())
			.map(|s| if !self.case_sensitive { s.to_lowercase() } else { s })
			.any(|mut s| {
				let mut negate = false;

				if s.starts_with('!') {
					negate = true;
					s = s.replacen('!', "", 1);
				}
				let mut matches = filename.contains(&s);
				if negate {
					matches = !matches
				}
				matches
			});

		startswith || endswith || contains
	}
}

#[cfg(test)]
mod tests {
	use std::str::FromStr;

	use super::*;

	#[test]
	fn match_beginning_case_insensitive() {
		let path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		let filename = Filename {
			startswith: vec!["TE".into()],
			..Default::default()
		};
		assert!(filename.matches(&path))
	}

	#[test]
	fn match_ending_case_insensitive() {
		let path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		let filename = Filename {
			endswith: vec!["DF".into()],
			..Default::default()
		};
		assert!(filename.matches(&path))
	}

	#[test]
	fn match_containing_case_insensitive() {
		let path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		let filename = Filename {
			contains: vec!["ES".into()],
			..Default::default()
		};
		assert!(filename.matches(&path))
	}

	#[test]
	fn no_match_beginning_case_sensitive() {
		let path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		let filename = Filename {
			case_sensitive: true,
			startswith: vec!["TE".into()],
			..Default::default()
		};
		assert!(!filename.matches(&path))
	}

	#[test]
	fn no_match_ending_case_sensitive() {
		let path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		let filename = Filename {
			case_sensitive: true,
			startswith: vec!["DF".into()],
			..Default::default()
		};
		assert!(!filename.matches(&path))
	}

	#[test]
	fn no_match_containing_case_sensitive() {
		let path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		let filename = Filename {
			case_sensitive: true,
			contains: vec!["ES".into()],
			..Default::default()
		};
		assert!(!filename.matches(&path))
	}
	#[test]
	fn match_containing_case_sensitive() {
		let path = Resource::from_str("$HOME/Downloads/tESt.pdf").unwrap();
		let filename = Filename {
			case_sensitive: true,
			contains: vec!["ES".into()],
			..Default::default()
		};
		assert!(filename.matches(&path))
	}
}
