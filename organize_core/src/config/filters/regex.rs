use crate::{config::filters::AsFilter, resource::Resource, templates::TERA};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Deserializer};
use std::{convert::TryFrom, ops::Deref, str::FromStr};

#[derive(PartialEq, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Regex {
	patterns: Vec<RegularExpression>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegularExpression {
	#[serde(deserialize_with = "deserialize_regex")]
	pub pattern: regex::Regex,
	#[serde(default)]
	pub negate: bool,
	pub input: String,
}

fn default_input() -> String {
	"{{path | filename}}".into()
}

impl Deref for RegularExpression {
	type Target = regex::Regex;

	fn deref(&self) -> &Self::Target {
		&self.pattern
	}
}

fn deserialize_regex<'de, D>(deserializer: D) -> Result<regex::Regex, D::Error>
where
	D: Deserializer<'de>,
{
	// Deserialize as a Vec<String>
	let patterns_str: String = String::deserialize(deserializer)?;
	regex::Regex::from_str(patterns_str.as_str()).map_err(serde::de::Error::custom)
}

impl PartialEq for RegularExpression {
	fn eq(&self, other: &Self) -> bool {
		self.pattern.as_str() == other.pattern.as_str()
	}
}
impl TryFrom<String> for RegularExpression {
	type Error = regex::Error;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		let pattern = regex::Regex::new(&value)?;
		Ok(Self {
			pattern,
			negate: false,
			input: default_input(),
		})
	}
}

impl AsFilter for RegularExpression {
	fn matches(&self, res: &Resource) -> bool {
		let input = TERA.lock().unwrap().render_str(&self.input, &res.context).unwrap();
		let mut matches = self.pattern.is_match(&input);
		if self.negate {
			matches = !matches;
		}
		matches
	}
}

impl AsFilter for Regex {
	fn matches(&self, res: &Resource) -> bool {
		self.patterns.par_iter().any(|f| f.matches(res))
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
