use crate::data::config::options::apply::{wrapper::ApplyWrapper, Apply};
use serde::{
	de::{Error, MapAccess, SeqAccess, Visitor},
	export::Formatter,
	Deserialize,
	Deserializer,
};
use std::{fmt, str::FromStr};

impl<'de> Deserialize<'de> for ApplyWrapper {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct ApplyVisitor;
		impl<'de> Visitor<'de> for ApplyVisitor {
			type Value = ApplyWrapper;

			fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
				formatter.write_str("string, seq or map")
			}

			fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
			where
				E: Error,
			{
				ApplyWrapper::from_str(v).map_err(E::custom)
			}

			fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
			where
				A: SeqAccess<'de>,
			{
				let mut vec = Vec::new();
				while let Some(val) = seq.next_element()? {
					vec.push(val)
				}
				Ok(ApplyWrapper::from(Apply::AllOf(vec)))
			}

			fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
			where
				M: MapAccess<'de>,
			{
				let mut wrapper = ApplyWrapper {
					actions: None,
					filters: None,
				};
				while let Some(key) = map.next_key::<String>()? {
					match key.as_str() {
						"actions" => {
							wrapper.actions = match wrapper.actions.is_some() {
								true => return Err(M::Error::duplicate_field("actions")),
								false => {
									let value = map.next_value()?;
									match value {
										Apply::All | Apply::AllOf(_) => Some(value),
										Apply::Any | Apply::AnyOf(_) => {
											return Err(M::Error::unknown_variant(&value.to_string(), &["all", "all_of"]))
										}
									}
								}
							}
						}
						"filters" => {
							wrapper.filters = match wrapper.filters.is_some() {
								true => return Err(M::Error::duplicate_field("filters")),
								false => Some(map.next_value()?),
							}
						}
						key => return Err(M::Error::unknown_field(key, &["actions", "filters"])),
					}
				}
				Ok(wrapper)
			}
		}
		deserializer.deserialize_any(ApplyVisitor)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_test::{assert_de_tokens, assert_de_tokens_error, Token};
	#[test]
	fn test_apply_wrapper_single_value_all() {
		let value = ApplyWrapper::from(Apply::All);
		assert_de_tokens(&value, &[Token::Str("all")])
	}

	#[test]
	fn test_apply_wrapper_single_value_any() {
		let value = ApplyWrapper::from(Apply::Any);
		assert_de_tokens(&value, &[Token::Str("any")])
	}

	#[test]
	fn test_apply_wrapper_single_value_vec() {
		let value = ApplyWrapper::from(Apply::AllOf(vec![0, 2]));
		assert_de_tokens(&value, &[Token::Seq { len: Some(2) }, Token::U8(0), Token::U8(2), Token::SeqEnd])
	}

	#[test]
	fn test_wrapper_unknown_field() {
		assert_de_tokens_error::<ApplyWrapper>(
			&[
				Token::Map { len: Some(2) },
				Token::Str("actions"),
				Token::Str("all"),
				Token::Str("unknown"),
				Token::MapEnd,
			],
			&serde::de::value::Error::unknown_field("unknown", &["actions", "filters"]).to_string(),
		)
	}

	#[test]
	fn test_apply_wrapper_actions_all_of_filters_all() {
		let value = ApplyWrapper {
			actions: Some(Apply::AllOf(vec![0, 1])),
			filters: Some(Apply::All),
		};
		assert_de_tokens(&value, &[
			Token::Map { len: Some(2) },
			Token::Str("actions"),
			Token::Map { len: Some(1) },
			Token::Str("all_of"),
			Token::Seq { len: Some(2) },
			Token::U8(0),
			Token::U8(1),
			Token::SeqEnd,
			Token::MapEnd,
			Token::Str("filters"),
			Token::Str("all"),
			Token::MapEnd,
		])
	}

	#[test]
	fn test_apply_wrapper_actions_all_filters_any() {
		let value = ApplyWrapper {
			actions: Some(Apply::All),
			filters: Some(Apply::Any),
		};
		assert_de_tokens(&value, &[
			Token::Map { len: Some(2) },
			Token::Str("actions"),
			Token::Str("all"),
			Token::Str("filters"),
			Token::Str("any"),
			Token::MapEnd,
		])
	}

	#[test]
	fn test_apply_wrapper_actions_all_filters_none() {
		let value = ApplyWrapper {
			actions: Some(Apply::All),
			filters: None,
		};
		assert_de_tokens(&value, &[
			Token::Map { len: Some(1) },
			Token::Str("actions"),
			Token::Str("all"),
			Token::MapEnd,
		])
	}

	#[test]
	fn test_apply_wrapper_actions_none_filters_all() {
		let value = ApplyWrapper {
			actions: None,
			filters: Some(Apply::All),
		};
		assert_de_tokens(&value, &[
			Token::Map { len: Some(1) },
			Token::Str("filters"),
			Token::Str("all"),
			Token::MapEnd,
		])
	}
	#[test]
	fn test_apply_wrapper_invalid_actions_any() {
		assert_de_tokens_error::<ApplyWrapper>(
			&[
				Token::Map { len: Some(2) },
				Token::Str("filters"),
				Token::Str("all"),
				Token::Str("actions"),
				Token::Str("any"),
				Token::MapEnd,
			],
			&serde::de::value::Error::unknown_variant("any", &["all", "all_of"]).to_string(),
		)
	}

	#[test]
	fn test_apply_wrapper_actions_any_of_filters_any() {
		assert_de_tokens_error::<ApplyWrapper>(
			&[
				Token::Map { len: Some(2) },
				Token::Str("filters"),
				Token::Str("all"),
				Token::Str("actions"),
				Token::Map { len: Some(1) },
				Token::Str("any_of"),
				Token::Seq { len: Some(1) },
				Token::U8(1),
				Token::SeqEnd,
				Token::MapEnd,
				Token::MapEnd,
			],
			&serde::de::value::Error::unknown_variant("any_of", &["all", "all_of"]).to_string(),
		)
	}
}
