use crate::config::options::apply::Apply;
use serde::{
	de::{Error, MapAccess, SeqAccess, Visitor},
	Deserialize, Deserializer,
};
use std::{fmt, fmt::Formatter, marker::PhantomData, str::FromStr};

impl<'de> Deserialize<'de> for Apply {
	fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
	where
		D: Deserializer<'de>,
	{
		struct ApplyVisitor(PhantomData<fn() -> Apply>);
		impl<'de> Visitor<'de> for ApplyVisitor {
			type Value = Apply;

			fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
				formatter.write_str("string, seq or map")
			}

			fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
			where
				E: Error,
			{
				Apply::from_str(v).map_err(E::custom)
			}

			fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
			where
				A: SeqAccess<'de>,
			{
				let mut vec = Vec::new();
				while let Some(val) = seq.next_element()? {
					vec.push(val)
				}
				Ok(Apply::AllOf(vec))
			}

			fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
			where
				A: MapAccess<'de>,
			{
				match map.next_key::<String>()? {
					Some(key) => match key.as_str() {
						"any_of" => Ok(Apply::AnyOf(map.next_value()?)),
						"all_of" => Ok(Apply::AllOf(map.next_value()?)),
						key => Err(A::Error::unknown_field(key, &["any_of", "all_of"])),
					},
					None => Err(A::Error::missing_field("any_of or all_of")),
				}
			}
		}
		deserializer.deserialize_any(ApplyVisitor(PhantomData))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_test::{assert_de_tokens, Token};

	#[test]
	fn test_apply_str_all() {
		let value = Apply::All;
		assert_de_tokens(&value, &[Token::Str("all")])
	}

	#[test]
	fn test_apply_str_any() {
		let value = Apply::Any;
		assert_de_tokens(&value, &[Token::Str("any")])
	}

	#[test]
	fn test_apply_str_vec() {
		let value = Apply::AllOf(vec![0, 1, 2]);
		assert_de_tokens(
			&value,
			&[Token::Seq { len: Some(3) }, Token::U8(0), Token::U8(1), Token::U8(2), Token::SeqEnd],
		)
	}

	#[test]
	fn test_apply_all_of() {
		let value = Apply::AllOf(vec![0, 1, 2]);
		assert_de_tokens(
			&value,
			&[
				Token::Map { len: Some(1) },
				Token::Str("all_of"),
				Token::Seq { len: Some(3) },
				Token::U8(0),
				Token::U8(1),
				Token::U8(2),
				Token::SeqEnd,
				Token::MapEnd,
			],
		)
	}

	#[test]
	fn test_apply_any_of() {
		let value = Apply::AnyOf(vec![0, 1, 2]);
		assert_de_tokens(
			&value,
			&[
				Token::Map { len: Some(1) },
				Token::Str("any_of"),
				Token::Seq { len: Some(3) },
				Token::U8(0),
				Token::U8(1),
				Token::U8(2),
				Token::SeqEnd,
				Token::MapEnd,
			],
		)
	}
}
