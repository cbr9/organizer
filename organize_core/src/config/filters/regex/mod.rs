mod de;

use std::{ops::Deref, path::Path, str::FromStr};

use crate::config::filters::AsFilter;

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
	fn matches(&self, path: &Path) -> bool {
		match path.file_name() {
			None => false,
			Some(filename) => {
				let filename = filename.to_string_lossy();
				self.iter().any(|re| re.is_match(&filename))
			}
		}
	}
}

impl From<Vec<&str>> for Regex {
	fn from(vec: Vec<&str>) -> Self {
		let vec = vec.iter().map(|str| regex::Regex::new(str).unwrap()).collect::<Vec<_>>();
		Self(vec)
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
		let path = Path::new("$HOME/Pictures/test_unsplash_img.jpg");
		assert!(regex.matches(path))
	}

	#[test]
	fn match_multiple() {
		let regex = Regex::from(vec![r".*unsplash.*", r"\w"]);
		let path = Path::new("$HOME/Pictures/test_unsplash_img.jpg");
		assert!(regex.matches(path))
	}

	#[test]
	fn no_match_multiple() {
		let regex = Regex::from(vec![r".*unsplash.*", r"\d"]);
		let path = Path::new("$HOME/Documents/deep_learning.pdf");
		assert!(!regex.matches(path))
	}
}
