use crate::config::filters::mime::{Mime, MimeWrapper};
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
				formatter.write_str("str")
			}

			fn visit_map<M>(self, mut map: M) -> Result<Regex, M::Error>
			where
				M: MapAccess<'de>,
			{
				let mut patterns = Vec::new();
				while let Some(key) = map.next_key::<String>()? {
					match key.as_str() {
						"types" => {
							let value = map.next_value::<Vec<String>>()?;
							patterns = value
								.into_iter()
								.map(|s| Mime::from_str(&s).map_err(M::Error::custom))
								.try_collect()?;
						}
						key => return Err(M::Error::unknown_field(key, &["types"])),
					}
				}
				Ok(MimeWrapper { patterns })
			}
		}
		deserializer.deserialize_any(WrapperVisitor)
	}
}
