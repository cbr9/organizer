use crate::config::filters::regex::Regex;
use serde::{
	de,
	de::{Error, SeqAccess, Visitor},
	export,
	Deserialize,
	Deserializer,
};
use std::{fmt, str::FromStr};

impl<'de> Deserialize<'de> for Regex {
	fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
	where
		D: Deserializer<'de>,
	{
		struct StringOrSeq;

		impl<'de> Visitor<'de> for StringOrSeq {
			type Value = Regex;

			fn expecting(&self, formatter: &mut export::Formatter) -> fmt::Result {
				formatter.write_str("string or seq")
			}

			fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				Regex::from_str(value).map_err(E::custom)
			}

			fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
			where
				A: SeqAccess<'de>,
			{
				let mut vec = Vec::new();
				while let Some(val) = seq.next_element::<String>()? {
					regex::Regex::new(&val).map_err(A::Error::custom).map(|re| {
						vec.push(re);
					})?;
				}
				Ok(Regex(vec))
			}
		}

		deserializer.deserialize_any(StringOrSeq)
	}
}

#[cfg(test)]
mod tests {
	use serde_test::{assert_de_tokens, Token};

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
}
