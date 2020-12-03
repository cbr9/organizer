use crate::data::config::filters::mime::{Mime, MimeWrapper};
use serde::{
	de::{Error, SeqAccess, Visitor},
	Deserialize, Deserializer,
};
use std::{fmt, str::FromStr};

impl<'de> Deserialize<'de> for Mime {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		Mime::from_str(&String::deserialize(deserializer)?).map_err(D::Error::custom)
	}
}

impl<'de> Deserialize<'de> for MimeWrapper {
	fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
	where
		D: Deserializer<'de>,
	{
		struct WrapperVisitor;
		impl<'de> Visitor<'de> for WrapperVisitor {
			type Value = MimeWrapper;

			fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
				formatter.write_str("str or seq")
			}

			fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
			where
				E: Error,
			{
				Ok(MimeWrapper(vec![Mime::from_str(v).map_err(E::custom)?]))
			}

			fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
			where
				A: SeqAccess<'de>,
			{
				let mut vec = Vec::new();
				while let Some(mime_type) = seq.next_element::<String>()? {
					vec.push(Mime::from_str(&mime_type).map_err(A::Error::custom)?);
				}
				Ok(MimeWrapper(vec))
			}
		}
		deserializer.deserialize_any(WrapperVisitor)
	}
}
