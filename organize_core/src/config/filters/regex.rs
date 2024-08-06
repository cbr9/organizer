use derive_more::Deref;

use crate::{config::filters::AsFilter, resource::Resource};
use serde::{Deserialize, Deserializer};
use std::{convert::TryFrom};

#[derive(PartialEq, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Regex {
	patterns: Vec<RegularExpression>,
}

#[derive(Debug, Deref, Clone)]
pub struct RegularExpression(regex::Regex);

impl<'de> Deserialize<'de> for RegularExpression {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		// Deserialize as a string first
		let pattern_str: String = String::deserialize(deserializer)?;
		// Attempt to compile the regular expression
		regex::Regex::new(&pattern_str).map(Self).map_err(serde::de::Error::custom)
	}
}
impl PartialEq for RegularExpression {
	fn eq(&self, other: &Self) -> bool {
		self.0.as_str() == other.0.as_str()
	}
}
impl TryFrom<String> for RegularExpression {
	type Error = regex::Error;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		regex::Regex::new(value.as_str()).map(Self)
	}
}

impl AsFilter for Regex {
	fn matches(&self, res: &Resource) -> bool {
		match res.path().as_ref().file_name() {
			None => false,
			Some(filename) => {
				let filename = filename.to_string_lossy();
				self.patterns.iter().any(|re| re.is_match(&filename))
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
		let mut path = Resource::from_str("$HOME/Pictures/test_unsplash_img.jpg").unwrap();
		assert!(regex.matches(&mut path))
	}

	#[test]
	fn match_multiple() {
		let regex = Regex::try_from(vec![r".*unsplash.*", r"\w"]).unwrap();
		let mut path = Resource::from_str("$HOME/Pictures/test_unsplash_img.jpg").unwrap();
		assert!(regex.matches(&mut path))
	}

	#[test]
	fn no_match_multiple() {
		let regex = Regex::try_from(vec![r".*unsplash.*", r"\d"]).unwrap();
		let mut path = Resource::from_str("$HOME/Documents/deep_learning.pdf").unwrap();
		assert!(!regex.matches(&mut path))
	}
}
