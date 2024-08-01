use std::{path::Path, str::FromStr};

use crate::config::filters::AsFilter;
use serde::{Deserialize, Deserializer};
use std::convert::TryFrom;

#[derive(Deserialize, Debug, Clone)]
pub struct Regex {
	#[serde(deserialize_with = "deserialize_patterns")]
	patterns: Vec<regex::Regex>,
}

fn deserialize_patterns<'de, D>(deserializer: D) -> Result<Vec<regex::Regex>, D::Error>
where
	D: Deserializer<'de>,
{
	// Deserialize as a Vec<String>
	let patterns_str: Vec<String> = Vec::deserialize(deserializer)?;
	Regex::try_from(patterns_str)
		.map(|o| o.patterns)
		.map_err(serde::de::Error::custom)
}

impl PartialEq for Regex {
	fn eq(&self, other: &Self) -> bool {
		self.patterns
			.iter()
			.zip(other.patterns.iter())
			.all(|(lhs, rhs)| lhs.as_str() == rhs.as_str())
	}
}
impl Eq for Regex {}

impl AsFilter for Regex {
	fn matches<T: AsRef<Path>>(&self, path: T) -> bool {
		match path.as_ref().file_name() {
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
			let re = regex::Regex::new(&str.to_string())?;
			vec.push(re)
		}
		Ok(Self { patterns: vec })
	}
}

impl FromStr for Regex {
	type Err = regex::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match regex::Regex::new(s) {
			Ok(regex) => Ok(Regex { patterns: vec![regex] }),
			Err(e) => Err(e),
		}
	}
}

#[cfg(test)]
mod tests {

	use super::*;

	#[test]
	fn match_single() {
		let regex = Regex::from_str(r".*unsplash.*").unwrap();
		let path = "$HOME/Pictures/test_unsplash_img.jpg";
		assert!(regex.matches(&path))
	}

	#[test]
	fn match_multiple() {
		let regex = Regex::try_from(vec![r".*unsplash.*", r"\w"]).unwrap();
		let path = "$HOME/Pictures/test_unsplash_img.jpg";
		assert!(regex.matches(&path))
	}

	#[test]
	fn no_match_multiple() {
		let regex = Regex::try_from(vec![r".*unsplash.*", r"\d"]).unwrap();
		let path = "$HOME/Documents/deep_learning.pdf";
		assert!(!regex.matches(&path))
	}
}
