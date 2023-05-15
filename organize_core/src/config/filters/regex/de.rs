use crate::config::filters::regex::Regex;
use itertools::Itertools;
use serde::{
	de::MapAccess,
	de::{Error, Visitor},
	Deserialize, Deserializer,
};
use std::{fmt, fmt::Formatter};

impl<'de> Deserialize<'de> for Regex {
	fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
	where
		D: Deserializer<'de>,
	{
		struct StringOrSeq;

		impl<'de> Visitor<'de> for StringOrSeq {
			type Value = Regex;

			fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
				formatter.write_str("map")
			}

			fn visit_map<M>(self, mut map: M) -> Result<Regex, M::Error>
			where
				M: MapAccess<'de>,
			{
				let mut patterns = Vec::new();
				while let Some(key) = map.next_key::<String>()? {
					match key.as_str() {
						"patterns" => {
							let value = map.next_value::<Vec<String>>()?;
							patterns = value
								.into_iter()
								.map(|s| regex::Regex::new(&s).map_err(M::Error::custom))
								.try_collect()?;
						}
						key => return Err(M::Error::unknown_field(key, &["patterns"])),
					}
				}
				Ok(Regex { patterns })
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
		let value = Regex { patterns: vec![re] };
		assert_de_tokens(&value, &[Token::Str(".*")])
	}

	#[test]
	fn deserialize_mult() {
		let first = regex::Regex::new(".*").unwrap();
		let sec = regex::Regex::new(".+").unwrap();
		let value = Regex { patterns: vec![first, sec] };
		assert_de_tokens(&value, &[Token::Seq { len: Some(2) }, Token::Str(".*"), Token::Str(".+"), Token::SeqEnd])
	}
}
