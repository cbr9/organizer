use super::FilterUtils;
use crate::{config::filters::AsFilter, resource::Resource, templates::Template};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::Deserialize;
use std::{convert::TryFrom, ops::Deref};

#[derive(PartialEq, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Regex {
	patterns: Vec<RegularExpression>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegularExpression {
	#[serde(deserialize_with = "serde_regex::deserialize")]
	pub pattern: regex::Regex,
	#[serde(default)]
	pub negate: bool,
	#[serde(default = "RegularExpression::default_input")]
	pub input: Template,
}

impl RegularExpression {
	fn default_input() -> Template {
		"{{path | filename}}".into()
	}
}

impl Deref for RegularExpression {
	type Target = regex::Regex;

	fn deref(&self) -> &Self::Target {
		&self.pattern
	}
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
			input: Self::default_input(),
		})
	}
}

impl AsFilter for RegularExpression {
	#[tracing::instrument(ret, level = "debug")]
	fn filter(&self, resources: &[&Resource]) -> Vec<bool> {
		resources
			.par_iter()
			.map(|res| {
				let input = self.input.render(&res.context).unwrap();
				let mut matches = self.pattern.is_match(&input);
				if self.negate {
					matches = !matches;
				}
				matches
			})
			.collect()
	}
}

impl AsFilter for Regex {
	fn filter(&self, resources: &[&Resource]) -> Vec<bool> {
		let results: Vec<Vec<bool>> = self.patterns.par_iter().map(|f| f.filter(resources)).collect();
		self.fold_vecs_with_any(results)
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
		assert_eq!(regex.filter(&[&path]), vec![true])
	}

	#[test]
	fn match_multiple() {
		let regex = Regex::try_from(vec![r".*unsplash.*", r"\w"]).unwrap();
		let path = Resource::from_str("$HOME/Pictures/test_unsplash_img.jpg").unwrap();
		assert_eq!(regex.filter(&[&path]), vec![true])
	}

	#[test]
	fn no_match_multiple() {
		let regex = Regex::try_from(vec![r".*unsplash.*", r"\d"]).unwrap();
		let path = Resource::from_str("$HOME/Documents/deep_learning.pdf").unwrap();
		assert_eq!(regex.filter(&[&path]), vec![false])
	}
}
