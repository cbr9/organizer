use crate::{
	data::config::actions::io_action::{ConflictOption, IOAction, Sep},
	path::Expand,
	string::visit_placeholder_string,
};
use serde::{
	de,
	de::{Error, MapAccess, Visitor},
	export::PhantomData,
	Deserialize, Deserializer,
};
use std::{fmt, path::PathBuf, str::FromStr};

impl<'de> Deserialize<'de> for IOAction {
	fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
	where
		D: Deserializer<'de>,
	{
		struct StringOrStruct(PhantomData<fn() -> IOAction>);

		impl<'de> Visitor<'de> for StringOrStruct {
			type Value = IOAction;

			fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
				formatter.write_str("string or map")
			}

			fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				let string = visit_placeholder_string(value).map_err(E::custom)?;
				IOAction::from_str(string.as_str()).map_err(E::custom)
			}

			fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
			where
				M: MapAccess<'de>,
			{
				let mut action: IOAction = IOAction::default();
				while let Some((key, value)) = map.next_entry::<String, String>()? {
					match key.as_str() {
						"to" => {
							action.to = match visit_placeholder_string(&value) {
								Ok(str) => {
									let path = PathBuf::from(str)
										.expand_vars()
										.map_err(M::Error::custom)?
										.expand_user()
										.map_err(M::Error::custom)?;
									if !path.exists() {
										return Err(M::Error::custom("path does not exist"));
									}
									path
								}
								Err(e) => return Err(M::Error::custom(e.to_string())),
							}
						}
						"if_exists" => {
							action.if_exists = match ConflictOption::from_str(&value) {
								Ok(value) => value,
								Err(e) => return Err(M::Error::custom(e)),
							}
						}
						"sep" => action.sep = Sep(value),
						_ => return Err(serde::de::Error::custom("unexpected key")),
					}
				}
				if action.to.to_string_lossy().is_empty() {
					return Err(serde::de::Error::custom("missing path"));
				}
				Ok(action)
			}
		}
		deserializer.deserialize_any(StringOrStruct(PhantomData))
	}
}
