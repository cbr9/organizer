use serde::{
	de::{Error, MapAccess, SeqAccess, Visitor},
	export::{Formatter, PhantomData},
	Deserialize,
	Deserializer,
	Serialize,
};
use std::fmt;

#[derive(Serialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum Apply {
	All,
	Any,
	Select(Vec<usize>),
}

#[derive(Debug, Serialize, Clone, Eq, PartialEq)]
pub struct ApplyWrapper {
	pub actions: Option<Apply>,
	pub filters: Option<Apply>,
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
					"all" => Ok(ApplyWrapper {
						filters: Some(Apply::All),
						actions: Some(Apply::All),
					}),
					"any" => Ok(ApplyWrapper {
						filters: Some(Apply::Any),
						actions: Some(Apply::All),
					}),
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
				Ok(ApplyWrapper {
					actions: Some(Apply::Select(vec.clone())),
					filters: Some(Apply::Select(vec)),
				})
			}

			fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
			where
				A: MapAccess<'de>,
			{
				let mut actions: Option<Apply> = None;
				let mut filters: Option<Apply> = None;

				while let Some(key) = map.next_key::<String>()? {
					match key.as_str() {
						"actions" => actions = Some(map.next_value()?),
						"filters" => filters = Some(map.next_value()?),
						_ => return Err(A::Error::custom("unknown field")),
					}
				}
				if let Some(actions) = &actions {
					if actions.eq(&Apply::Any) {
						return Err(A::Error::custom(
							"variant 'any' not valid for field 'actions' in option 'apply' (select 'all' or provide an array of indices)",
						));
					}
				}
				Ok(ApplyWrapper { actions, filters })
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
