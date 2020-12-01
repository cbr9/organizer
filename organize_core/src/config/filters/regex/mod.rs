mod de;

use std::{ops::Deref, path::Path, str::FromStr};

use crate::config::filters::AsFilter;
use std::convert::TryFrom;

#[derive(Debug, Clone)]
pub struct Regex(pub Vec<regex::Regex>);

impl PartialEq for Regex {
	fn eq(&self, other: &Self) -> bool {
		self.iter().zip(other.iter()).all(|(lhs, rhs)| lhs.as_str() == rhs.as_str())
	}
}

impl Deref for Regex {
	type Target = Vec<regex::Regex>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl AsFilter for Regex {
	fn matches<T: AsRef<Path>>(&self, path: &T) -> bool {
		match path.as_ref().file_name() {
			None => false,
			Some(filename) => {
				let filename = filename.to_string_lossy();
				self.iter().any(|re| re.is_match(&filename))
			}
		}
	}
}

impl TryFrom<Vec<&str>> for Regex {
	type Error = regex::Error;

	fn try_from(value: Vec<&str>) -> Result<Self, Self::Error> {
		let mut vec = Vec::with_capacity(value.len());
		for str in value {
			let re = regex::Regex::new(str)?;
			vec.push(re)
		}
		Ok(Self(vec))
	}
}

impl FromStr for Regex {
	type Err = regex::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match regex::Regex::new(s) {
			Ok(regex) => Ok(Regex(vec![regex])),
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
