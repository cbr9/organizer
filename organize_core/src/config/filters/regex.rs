use crate::{config::filters::AsFilter, resource::Resource};
use serde::{Deserialize, Deserializer};
use std::{convert::TryFrom, ops::Deref};

#[derive(PartialEq, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Regex {
	patterns: Vec<RegularExpression>,
}

#[derive(Debug, Clone)]
pub struct RegularExpression {
	pattern: regex::Regex,
	negate: bool,
}

impl Deref for RegularExpression {
	type Target = regex::Regex;

	fn deref(&self) -> &Self::Target {
		&self.pattern
	}
}
impl<'de> Deserialize<'de> for RegularExpression {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		// Deserialize as a string first
		let pattern_str: String = String::deserialize(deserializer)?;
		Self::try_from(pattern_str).map_err(serde::de::Error::custom)
	}
}
impl PartialEq for RegularExpression {
	fn eq(&self, other: &Self) -> bool {
		self.pattern.as_str() == other.pattern.as_str()
	}
}
impl TryFrom<String> for RegularExpression {
	type Error = regex::Error;

	fn try_from(mut value: String) -> Result<Self, Self::Error> {
		let mut negate = false;
		if value.starts_with('!') {
			negate = true;
			value = value.replacen('!', "", 1);
		}

		if value.starts_with("\\!") {
			value = value.replacen('\\', "", 1);
		}

		let pattern = regex::Regex::new(&value)?;
		Ok(Self { pattern, negate })
	}
}

impl AsFilter for Regex {
	fn matches(&self, res: &Resource) -> bool {
		match res.path.file_name() {
			None => false,
			Some(filename) => {
				let filename = filename.to_string_lossy();
				self.patterns.iter().any(|re| {
					let mut matches = re.is_match(&filename);
					if re.negate {
						matches = !matches;
					}
					matches
				})
			}
		}
	}
}

impl<T: ToString> TryFrom<Vec<T>> for Regex {
	type Error = regex::Error;

	fn try_from(value: Vec<T>) -> Result<Self, Self::Error> {
		let mut vec = Vec::with_capacity(value.len());
		for str in value {
			let re = RegularExpression::try_from(str.to_string())?;
			vec.push(re)
		}
		Ok(Self { patterns: vec })
	}
}

#[cfg(test)]
mod tests {

	use std::str::FromStr;

	use super::*;

	#[test]
	fn match_single() {
		let regex = Regex::try_from(vec![r".*unsplash.*"]).unwrap();
		let path = Resource::from_str("$HOME/Pictures/test_unsplash_img.jpg").unwrap();
		assert!(regex.matches(&path))
	}

	#[test]
	fn match_multiple() {
		let regex = Regex::try_from(vec![r".*unsplash.*", r"\w"]).unwrap();
		let path = Resource::from_str("$HOME/Pictures/test_unsplash_img.jpg").unwrap();
		assert!(regex.matches(&path))
	}

	#[test]
	fn no_match_multiple() {
		let regex = Regex::try_from(vec![r".*unsplash.*", r"\d"]).unwrap();
		let path = Resource::from_str("$HOME/Documents/deep_learning.pdf").unwrap();
		assert!(!regex.matches(&path))
	}
}
