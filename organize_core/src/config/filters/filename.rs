use crate::{config::filters::AsFilter, resource::Resource, templates::Template};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::Deserialize;

// TODO: refactor

#[derive(Eq, PartialEq, Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Filename {
	#[serde(default)]
	pub startswith: Vec<Template>,
	#[serde(default)]
	pub endswith: Vec<Template>,
	#[serde(default)]
	pub contains: Vec<Template>,
	#[serde(default)]
	pub case_sensitive: bool,
}

impl AsFilter for Filename {
	#[tracing::instrument(ret, level = "debug")]
	fn filter(&self, resources: &[&Resource]) -> Vec<bool> {
		resources
			.par_iter()
			.map(|res| {
				let filename = res.path.file_name().unwrap_or_default().to_string_lossy();

				if filename.is_empty() {
					return false;
				}

				let startswith = if self.startswith.is_empty() {
					true
				} else {
					self.startswith
						.iter()
						.map(|s| {
							s.render(&res.context)
								.map(|s| if !self.case_sensitive { s.to_lowercase() } else { s })
						})
						.flatten()
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
						})
				};

				let endswith = if self.endswith.is_empty() {
					true
				} else {
					self.endswith
						.iter()
						.map(|s| {
							s.render(&res.context)
								.map(|s| if !self.case_sensitive { s.to_lowercase() } else { s })
						})
						.flatten()
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
						})
				};

				let contains = if self.contains.is_empty() {
					true
				} else {
					self.contains
						.iter()
						.map(|s| {
							s.render(&res.context)
								.map(|s| if !self.case_sensitive { s.to_lowercase() } else { s })
						})
						.flatten()
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
						})
				};
				startswith && endswith && contains
			})
			.collect()
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
		assert_eq!(filename.filter(&[&path]), vec![true])
	}

	#[test]
	fn match_ending_case_insensitive() {
		let path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		let filename = Filename {
			endswith: vec!["DF".into()],
			..Default::default()
		};
		assert_eq!(filename.filter(&[&path]), vec![true])
	}

	#[test]
	fn match_containing_case_insensitive() {
		let path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		let filename = Filename {
			contains: vec!["ES".into()],
			..Default::default()
		};
		assert_eq!(filename.filter(&[&path]), vec![true])
	}

	#[test]
	fn no_match_beginning_case_sensitive() {
		let path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		let filename = Filename {
			case_sensitive: true,
			startswith: vec!["TE".into()],
			..Default::default()
		};
		assert_eq!(filename.filter(&[&path]), vec![false])
	}

	#[test]
	fn no_match_ending_case_sensitive() {
		let path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		let filename = Filename {
			case_sensitive: true,
			startswith: vec!["DF".into()],
			..Default::default()
		};
		assert_eq!(filename.filter(&[&path]), vec![false])
	}

	#[test]
	fn no_match_containing_case_sensitive() {
		let path = Resource::from_str("$HOME/Downloads/test.pdf").unwrap();
		let filename = Filename {
			case_sensitive: true,
			contains: vec!["ES".into()],
			..Default::default()
		};
		assert_eq!(filename.filter(&[&path]), vec![false])
	}
	#[test]
	fn match_containing_case_sensitive() {
		let path = Resource::from_str("$HOME/Downloads/tESt.pdf").unwrap();
		let filename = Filename {
			case_sensitive: true,
			contains: vec!["ES".into()],
			..Default::default()
		};
		assert_eq!(filename.filter(&[&path]), vec![true])
	}
	#[test]
	fn match_multiple_conditions_case_sensitive() {
		let path = Resource::from_str("$HOME/Downloads/tESt.pdf").unwrap();
		let filename = Filename {
			case_sensitive: true,
			contains: vec!["ES".into()],
			startswith: vec!["t".into()],
			endswith: vec!["df".into()],
		};
		assert_eq!(filename.filter(&[&path]), vec![true])
	}
	#[test]
	fn match_multiple_conditions_some_negative() {
		let path = Resource::from_str("$HOME/Downloads/tESt.pdf").unwrap();
		let filename = Filename {
			case_sensitive: true,
			contains: vec!["ES".into()],
			startswith: vec!["t".into()],
			endswith: vec!["!df".into()],
		};
		assert_eq!(filename.filter(&[&path]), vec![false])
	}
	#[test]
	fn match_multiple_conditions_some_negative_2() {
		let path = Resource::from_str("$HOME/Downloads/tESt.pdf").unwrap();
		let filename = Filename {
			case_sensitive: true,
			contains: vec!["!ES".into(), "ES".into()],
			startswith: vec!["t".into()],
			endswith: vec!["!df".into()],
		};
		assert_eq!(filename.filter(&[&path]), vec![false])
	}
	#[test]
	fn match_multiple_conditions_some_negative_3() {
		let path = Resource::from_str("$HOME/Downloads/tESt.txt").unwrap();
		let filename = Filename {
			case_sensitive: true,
			contains: vec!["!ES".into(), "ES".into()],
			startswith: vec!["t".into()],
			endswith: vec!["!df".into()],
		};
		assert_eq!(filename.filter(&[&path]), vec![true])
	}
}
