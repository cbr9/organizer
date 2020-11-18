use std::fmt;

use crate::config::AsOption;
use serde::{
	de::{Error, MapAccess, SeqAccess, Unexpected, VariantAccess, Visitor},
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
	AllOf(Vec<usize>),
	AnyOf(Vec<usize>),
}

impl Default for Apply {
	fn default() -> Self {
		Self::All
	}
}

impl FromStr for Apply {
	type Err = serde::de::value::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"all" => Ok(Self::All),
			"any" => Ok(Self::Any),
			_ => Err(serde::de::value::Error::unknown_variant(s, &["all", "any"])),
		}
	}
}

impl AsOption<Apply> for Option<Apply> {
	fn combine(self, rhs: Self) -> Self {
		match (self, rhs) {
			(None, None) => Some(Apply::default()),
			(Some(lhs), None) => Some(lhs),
			(None, Some(rhs)) => Some(rhs),
			(Some(Apply::AllOf(mut lhs)), Some(Apply::AllOf(mut rhs))) => {
				rhs.append(&mut lhs);
				rhs.sort_unstable();
				rhs.dedup();
				Some(Apply::AllOf(rhs))
			}
			(Some(Apply::AnyOf(mut lhs)), Some(Apply::AnyOf(mut rhs))) => {
				rhs.append(&mut lhs);
				rhs.sort_unstable();
				rhs.dedup();
				Some(Apply::AnyOf(rhs))
			}
			(_, rhs) => rhs,
		}
	}
}

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
			Apply::AllOf(_) => "all_of".into(),
			Apply::AnyOf(_) => "any_of".into(),
		}
	}
}

#[derive(Debug, Serialize, Clone, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ApplyWrapper {
	pub actions: Option<Apply>,
	pub filters: Option<Apply>,
}

impl Default for ApplyWrapper {
	fn default() -> Self {
		Self {
			actions: Some(Apply::default()),
			filters: Some(Apply::default()),
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
			Apply::AllOf(vec) => Self {
				actions: Some(Apply::AllOf(vec.clone())),
				filters: Some(Apply::AllOf(vec)),
			},
			Apply::AnyOf(vec) => Self {
				actions: Some(Apply::AllOf(vec.clone())),
				filters: Some(Apply::AnyOf(vec)),
			},
		}
	}
}

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

impl FromStr for ApplyWrapper {
	type Err = serde::de::value::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Self::from(Apply::from_str(s)?))
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
			}
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
	fn test_apply_str_vec() {
		let value = Apply::AllOf(vec![0, 1, 2]);
		assert_de_tokens(&value, &[
			Token::Seq { len: Some(3) },
			Token::U8(0),
			Token::U8(1),
			Token::U8(2),
			Token::SeqEnd,
		])
	}

	#[test]
	fn test_apply_all_of() {
		let value = Apply::AllOf(vec![0, 1, 2]);
		assert_de_tokens(&value, &[
			Token::Map { len: Some(1) },
			Token::Str("all_of"),
			Token::Seq { len: Some(3) },
			Token::U8(0),
			Token::U8(1),
			Token::U8(2),
			Token::SeqEnd,
			Token::MapEnd,
		])
	}

	#[test]
	fn test_apply_any_of() {
		let value = Apply::AnyOf(vec![0, 1, 2]);
		assert_de_tokens(&value, &[
			Token::Map { len: Some(1) },
			Token::Str("any_of"),
			Token::Seq { len: Some(3) },
			Token::U8(0),
			Token::U8(1),
			Token::U8(2),
			Token::SeqEnd,
			Token::MapEnd,
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

	#[test]
	fn combine_apply_some_some() {
		let left = Some(Apply::All);
		let right = Some(Apply::Any);
		assert_eq!(left.combine(right.clone()), right)
	}

	#[test]
	fn combine_apply_some_none() {
		let left = Some(Apply::All);
		let right = None;
		assert_eq!(left.clone().combine(right), left)
	}

	#[test]
	fn combine_apply_none_some() {
		let left = None;
		let right = Some(Apply::All);
		assert_eq!(left.combine(right.clone()), right)
	}

	#[test]
	fn combine_apply_vec_some_some_all_of_all_of() {
		let left = Some(Apply::AllOf(vec![0, 1]));
		let right = Some(Apply::AllOf(vec![2]));
		let expected = Some(Apply::AllOf(vec![0, 1, 2]));
		assert_eq!(left.combine(right), expected)
	}

	#[test]
	fn combine_apply_vec_some_some_any_of_any_of() {
		let left = Some(Apply::AnyOf(vec![0, 1]));
		let right = Some(Apply::AnyOf(vec![2]));
		let expected = Some(Apply::AnyOf(vec![0, 1, 2]));
		assert_eq!(left.combine(right), expected)
	}

	#[test]
	fn combine_apply_vec_some_some_all_of_any_of() {
		let left = Some(Apply::AllOf(vec![0, 1]));
		let right = Some(Apply::AnyOf(vec![2]));
		assert_eq!(left.combine(right.clone()), right)
	}

	#[test]
	fn combine_apply_vec_some_some_any_of_all_of() {
		let left = Some(Apply::AnyOf(vec![2]));
		let right = Some(Apply::AllOf(vec![0, 1]));
		assert_eq!(left.combine(right.clone()), right)
	}

	#[test]
	fn combine_apply_vec_some_none() {
		let left = Some(Apply::All);
		let right = None;
		assert_eq!(left.clone().combine(right), left)
	}

	#[test]
	fn combine_apply_vec_none_some() {
		let left = None;
		let right = Some(Apply::All);
		assert_eq!(left.combine(right.clone()), right)
	}

	#[test]
	fn combine_apply_none_none() {
		let left: Option<Apply> = None;
		let right = None;
		assert_eq!(left.combine(right), Some(Apply::default()))
	}

	#[test]
	fn combine_wrapper_none_none() {
		let left: Option<ApplyWrapper> = None;
		let right = None;
		assert_eq!(left.combine(right), Some(ApplyWrapper::default()))
	}

	#[test]
	fn combine_wrapper_none_some() {
		let left: Option<ApplyWrapper> = None;
		let right = Some(ApplyWrapper::default());
		assert_eq!(left.combine(right.clone()), right)
	}

	#[test]
	fn combine_wrapper_some_some() {
		let left: Option<ApplyWrapper> = Some(ApplyWrapper {
			actions: Some(Apply::Any),
			filters: None,
		});
		let right = Some(ApplyWrapper {
			actions: None,
			filters: None,
		});
		let expected = Some(ApplyWrapper {
			actions: Some(Apply::Any),
			filters: Some(Apply::All),
		});
		assert_eq!(left.combine(right), expected)
	}
}
