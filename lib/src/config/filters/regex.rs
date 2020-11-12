use std::{fmt, ops::Deref, path::Path, str::FromStr};

use crate::config::{filters::Extension, AsFilter};
use serde::{
	de,
	de::{Error, SeqAccess, Visitor},
	export,
	export::PhantomData,
	Deserialize,
	Deserializer,
};

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
		struct StringOrSeq(PhantomData<fn() -> Regex>);

		impl<'de> Visitor<'de> for StringOrSeq {
			type Value = Regex;

			fn expecting(&self, formatter: &mut export::Formatter) -> fmt::Result {
				formatter.write_str("string or seq")
			}

			fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				Regex::from_str(value).or_else(|_| Err(E::custom("invalid regex")))
			}

			fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
			where
				A: SeqAccess<'de>,
			{
				let mut vec = Vec::new();
				while let Some(val) = seq.next_element::<String>()? {
					match regex::Regex::new(&val) {
						Ok(re) => vec.push(re),
						Err(_) => return Err(A::Error::custom("invalid regex")),
					}
				}
				Ok(Regex(vec))
			}
		}

		deserializer.deserialize_any(StringOrSeq(PhantomData))
	}
}

#[cfg(test)]
mod tests {
	use serde_test::{assert_de_tokens, Token};
	use std::io::{Error, ErrorKind};

	use super::*;

	#[test]
	fn deserialize_single() {
		let re = regex::Regex::new(".*").unwrap();
		let value = Regex(vec![re]);
		assert_de_tokens(&value, &[Token::Str(".*")])
	}

	#[test]
	fn deserialize_mult() {
		let first = regex::Regex::new(".*").unwrap();
		let sec = regex::Regex::new(".+").unwrap();
		let value = Regex(vec![first, sec]);
		assert_de_tokens(&value, &[Token::Seq { len: Some(2) }, Token::Str(".*"), Token::Str(".+"), Token::SeqEnd])
	}

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
