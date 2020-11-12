use std::fmt;

use crate::config::AsOption;
use serde::{
	de::{Error, MapAccess, SeqAccess, Visitor},
	export::{Formatter, PhantomData},
	Deserialize,
	Deserializer,
	Serialize,
};
use std::str::FromStr;

#[derive(Serialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum Apply {
	All,
	Any,
	Select(Vec<usize>),
}

impl Default for Apply {
	fn default() -> Self {
		Self::All
	}
}

impl FromStr for Apply {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"all" => Ok(Self::All),
			"any" => Ok(Self::Any),
			_ => Err("invalid value".into()),
		}
	}
}

impl AsOption<Apply> for Option<Apply> {
	fn combine(self, rhs: Self) -> Self {
		match (self, rhs) {
			(None, None) => Some(Apply::default()),
			(Some(lhs), None) => Some(lhs),
			(None, Some(rhs)) => Some(rhs),
			(Some(_), Some(rhs)) => Some(rhs),
		}
	}
}

#[derive(Debug, Serialize, Clone, Eq, PartialEq)]
pub struct ApplyWrapper {
	pub actions: Option<Apply>,
	pub filters: Option<Apply>,
}

impl Default for ApplyWrapper {
	fn default() -> Self {
		Self {
			actions: Some(Apply::All),
			filters: Some(Apply::All),
		}
	}
}

impl From<Apply> for ApplyWrapper {
	fn from(val: Apply) -> Self {
		match val {
			Apply::All => Self {
				actions: Some(val.clone()),
				filters: Some(val),
			},
			Apply::Any => Self {
				actions: Some(Apply::All),
				filters: Some(val),
			},
			Apply::Select(vec) => Self {
				actions: Some(Apply::Select(vec.clone())),
				filters: Some(Apply::Select(vec)),
			},
		}
	}
}

impl<'de> Deserialize<'de> for Apply {
	fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
	where
		D: Deserializer<'de>,
	{
		struct ApplyVisitor;
		impl<'de> Visitor<'de> for ApplyVisitor {
			type Value = Apply;

			fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
				formatter.write_str("string or seq")
			}

			fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
			where
				E: Error,
			{
				match v {
					"all" => Ok(Apply::All),
					"any" => Ok(Apply::Any),
					_ => Err(E::custom("unknown variant")),
				}
			}

			fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
			where
				A: SeqAccess<'de>,
			{
				let mut vec = Vec::new();
				while let Some(val) = seq.next_element()? {
					vec.push(val)
				}
				Ok(Apply::Select(vec))
			}
		}
		deserializer.deserialize_any(ApplyVisitor)
	}
}

impl<'de> Deserialize<'de> for ApplyWrapper {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct ApplyVisitor(PhantomData<fn() -> ApplyWrapper>);
		impl<'de> Visitor<'de> for ApplyVisitor {
			type Value = ApplyWrapper;

			fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
				formatter.write_str("string, seq or map")
			}

			fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
			where
				E: Error,
			{
				match v {
					"all" => Ok(ApplyWrapper::from(Apply::All)),
					"any" => Ok(ApplyWrapper::from(Apply::Any)),
					_ => Err(E::custom("unknown option")),
				}
			}

			fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
			where
				A: SeqAccess<'de>,
			{
				let mut vec = Vec::new();
				while let Some(val) = seq.next_element()? {
					vec.push(val)
				}
				Ok(ApplyWrapper::from(Apply::Select(vec)))
			}

			fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
			where
				A: MapAccess<'de>,
			{
				let mut wrapper = ApplyWrapper {
					actions: None,
					filters: None,
				};

				while let Some((key, value)) = map.next_entry::<String, Apply>()? {
					match key.as_str() {
						"actions" => {
							wrapper.actions = match value {
								Apply::All => Some(value),
								Apply::Select(_) => Some(value),
								Apply::Any => {
									let msg = "variant 'any' not valid for field 'actions' in option 'apply'";
									return Err(A::Error::custom(msg));
								}
							}
						}
						"filters" => wrapper.filters = Some(value),
						_ => return Err(A::Error::custom("unknown field")),
					}
				}
				Ok(wrapper)
			}
		}
		deserializer.deserialize_any(ApplyVisitor(PhantomData))
	}
}

impl AsRef<Self> for Apply {
	fn as_ref(&self) -> &Self {
		self
	}
}

impl ToString for Apply {
	fn to_string(&self) -> String {
		match self {
			Apply::All => "all".into(),
			Apply::Any => "any".into(),
			Apply::Select(vec) => format!("{:?}", vec),
		}
	}
}
impl AsOption<ApplyWrapper> for Option<ApplyWrapper> {
	fn combine(self, rhs: Self) -> Self
	where
		Self: Sized,
	{
		match (self, rhs) {
			(None, Some(rhs)) => Some(rhs),
			(Some(lhs), None) => Some(lhs),
			(None, None) => Some(ApplyWrapper::default()),
			(Some(lhs), Some(rhs)) => {
				let wrapper = ApplyWrapper {
					actions: lhs.actions.combine(rhs.actions),
					filters: lhs.filters.combine(rhs.filters),
				};
				Some(wrapper)
			},
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_test::{assert_de_tokens, assert_de_tokens_error, Token};

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
	fn test_apply_select() {
		let value = Apply::Select(vec![0, 1, 2]);
		assert_de_tokens(&value, &[
			Token::Seq { len: Some(3) },
			Token::U8(0),
			Token::U8(1),
			Token::U8(2),
			Token::SeqEnd,
		])
	}
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
	fn test_apply_wrapper_single_value_select() {
		let value = ApplyWrapper::from(Apply::Select(vec![0, 2]));
		assert_de_tokens(&value, &[Token::Seq { len: Some(2) }, Token::U8(0), Token::U8(2), Token::SeqEnd])
	}
	#[test]
	fn test_apply_wrapper_actions_select_filters_all() {
		let value = ApplyWrapper {
			actions: Some(Apply::Select(vec![0, 1])),
			filters: Some(Apply::All),
		};
		assert_de_tokens(&value, &[
			Token::Map { len: Some(2) },
			Token::Str("actions"),
			Token::Seq { len: Some(2) },
			Token::U8(0),
			Token::U8(1),
			Token::SeqEnd,
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
	fn test_apply_wrapper_actions_none_filters_none() {
		let value = ApplyWrapper {
			actions: None,
			filters: None,
		};
		assert_de_tokens(&value, &[Token::Map { len: None }, Token::MapEnd])
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
	fn test_apply_wrapper_actions_any_filters_any() {
		assert_de_tokens_error::<ApplyWrapper>(
			&[
				Token::Map { len: Some(2) },
				Token::Str("filters"),
				Token::Str("all"),
				Token::Str("actions"),
				Token::Str("any"),
				Token::MapEnd,
			],
			"variant 'any' not valid for field 'actions' in option 'apply' (select 'all' or provide an array of indices)",
		)
	}
}
