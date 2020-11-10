use std::{ops::Deref, path::Path, str::FromStr};

use crate::config::{filters::Extension, AsFilter};
use serde::{de::Error, Deserialize, Deserializer};

#[derive(Debug, Clone)]
pub struct Regex(pub Vec<regex::Regex>);

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
				for regex in self.iter() {
					if regex.is_match(&filename.to_string_lossy()) {
						return true;
					}
				}
				false
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

impl<'de> Deserialize<'de> for Regex {
	fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
	where
		D: Deserializer<'de>,
	{
		let vec = Extension::deserialize(deserializer)?; // the Extension deserializer is a plain String or Vec deserializer
		let mut regexes = Vec::new();
		for str in vec.iter() {
			match regex::Regex::new(str) {
				Ok(regex) => regexes.push(regex),
				Err(_) => return Err(D::Error::custom("invalid regex")),
			}
		}
		Ok(Regex(regexes))
	}
}

#[cfg(test)]
mod tests {
	use std::io::{Error, ErrorKind, Result};

	use crate::utils::tests::IntoResult;

	use super::*;

	#[test]
	fn deserialize_single() -> Result<()> {
		serde_yaml::from_str::<Regex>(".*").map_or_else(|e| Err(Error::new(ErrorKind::Other, e.to_string())), |_| Ok(()))
	}

	#[test]
	fn deserialize_mult() -> Result<()> {
		serde_yaml::from_str::<Regex>("[.*]").map_or_else(|e| Err(Error::new(ErrorKind::Other, e.to_string())), |_| Ok(()))
	}

	#[test]
	fn match_single() -> Result<()> {
		let regex = Regex::from_str(r".*unsplash.*").map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
		let path = Path::new("$HOME/Pictures/test_unsplash_img.jpg");
		regex.matches(path).into_result()
	}

	#[test]
	fn match_multiple() -> Result<()> {
		let regex = Regex::from(vec![r".*unsplash.*", r"\w"]);
		let path = Path::new("$HOME/Pictures/test_unsplash_img.jpg");
		regex.matches(path).into_result()
	}

	#[test]
	#[should_panic]
	fn no_match_multiple() {
		let regex = Regex::from(vec![r".*unsplash.*", r"\d"]);
		let path = Path::new("$HOME/Documents/deep_learning.pdf");
		regex.matches(path).into_result().unwrap()
	}
}
