use crate::config::filters::AsFilter;
use mime::FromStrError;
use serde::{Deserialize, Deserializer};
use std::{convert::TryFrom, path::Path, str::FromStr};

impl FromStr for Mime {
	type Err = FromStrError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Mime {
			types: vec![mime::Mime::from_str(s)?],
		})
	}
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
pub struct Mime {
	#[serde(deserialize_with = "deserialize_mimetypes")]
	types: Vec<mime::Mime>,
}

fn deserialize_mimetypes<'de, D>(deserializer: D) -> Result<Vec<mime::Mime>, D::Error>
where
	D: Deserializer<'de>,
{
	// Deserialize as a Vec<String>
	let patterns_str: Vec<String> = Vec::deserialize(deserializer)?;
	Mime::try_from(patterns_str).map(|o| o.types).map_err(serde::de::Error::custom)
}

impl<T: ToString> TryFrom<Vec<T>> for Mime {
	type Error = FromStrError;

	fn try_from(value: Vec<T>) -> Result<Self, Self::Error> {
		let mut vec = Vec::with_capacity(value.len());
		for str in value {
			match mime::Mime::from_str(&str.to_string()) {
				Ok(mime) => vec.push(mime),
				Err(e) => return Err(e),
			}
		}
		Ok(Mime { types: vec })
	}
}

impl AsFilter for Mime {
	fn matches<T: AsRef<Path>>(&self, path: T) -> bool {
		let guess = mime_guess::from_path(path.as_ref()).first_or_octet_stream();
		self.types.iter().any(|mime| match (mime.type_(), mime.subtype()) {
			(mime::STAR, subtype) => subtype == guess.subtype(),
			(type_, mime::STAR) => type_ == guess.type_(),
			(type_, subtype) => type_ == guess.type_() && subtype == guess.subtype(),
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn test_match() {
		let types = Mime::try_from(vec!["image/*", "audio/*"]).unwrap();
		let img = "test.jpg";
		let audio = "test.ogg";
		assert!(types.matches(img));
		assert!(types.matches(audio));
	}
}
