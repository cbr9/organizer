use std::{fmt, path::PathBuf, str::FromStr};

use serde::{
	de,
	de::{Error, MapAccess, Visitor},
	Deserialize,
	Deserializer,
};

use crate::{
	data::config::actions::io_action::{ConflictOption, Inner},
	path::Expand,
	string::visit_placeholder_string,
};
use std::marker::PhantomData;

impl<'de> Deserialize<'de> for Inner {
	fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
	where
		D: Deserializer<'de>,
	{
		struct StringOrStruct(PhantomData<fn() -> Inner>);

		impl<'de> Visitor<'de> for StringOrStruct {
			type Value = Inner;

			fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
				formatter.write_str("string or map")
			}

			fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				let string = visit_placeholder_string(value).map_err(E::custom)?;
				Inner::from_str(string.as_str()).map_err(E::custom)
			}

			fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
			where
				M: MapAccess<'de>,
			{
				let mut to: Option<PathBuf> = None;
				let mut if_exists: Option<ConflictOption> = None;
				while let Some((key, value)) = map.next_entry::<String, String>()? {
					match key.as_str() {
						"to" => {
							to = {
								let path = visit_placeholder_string(&value)
									.map(|str| -> Result<PathBuf, M::Error> {
										let path = PathBuf::from(str)
											.expand_vars()
											.map_err(M::Error::custom)?
											.expand_user()
											.map_err(M::Error::custom)?;
										Ok(path)
									})
									.map_err(M::Error::custom)??;
								Some(path)
							};
						}
						"if_exists" => if_exists = Some(ConflictOption::from_str(&value).map_err(M::Error::custom)?),
						other => return Err(M::Error::unknown_field(other, &["to", "if_exists"])),
					}
				}
				let action = Inner {
					to: to.ok_or_else(|| M::Error::missing_field("to"))?,
					if_exists: if_exists.unwrap_or_default(),
				};
				Ok(action)
			}
		}
		deserializer.deserialize_any(StringOrStruct(PhantomData))
	}
}

#[cfg(test)]
mod tests {
	use serde_test::{assert_de_tokens, Token};

	use super::*;
	use dirs::home_dir;

	#[test]
	fn deserialize_str() {
		let value = Inner {
			to: home_dir().unwrap(),
			if_exists: Default::default(),
		};
		assert_de_tokens(&value, &[Token::Str("$HOME")])
	}

	#[test]
	fn deserialize_map() {
		let value = Inner {
			to: home_dir().unwrap(),
			if_exists: ConflictOption::Rename {
				counter_separator: "-".into(),
			},
		};
		assert_de_tokens(&value, &[
			Token::Map { len: Some(3) },
			Token::Str("to"),
			Token::Str("$HOME"),
			Token::Str("if_exists"),
			Token::Str("rename with \"-\""),
			Token::MapEnd,
		])
	}
}
