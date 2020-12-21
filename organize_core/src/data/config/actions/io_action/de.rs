use std::{fmt, path::PathBuf, str::FromStr};
use std::ops::Deref;

use serde::{
	de,
	de::{Error, MapAccess, Visitor},
	Deserialize,
	Deserializer, export::PhantomData,
};

use crate::{
	data::config::actions::io_action::{ConflictOption, IOAction, Sep},
	path::Expand,
	string::visit_placeholder_string,
};

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
				let mut to: Option<PathBuf> = None;
				let mut if_exists: Option<ConflictOption> = None;
				let mut sep: Option<Sep> = None;
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
						"sep" => sep = Some(Sep(value)),
						other => return Err(M::Error::unknown_field(other, &["to", "if_exists", "sep"])),
					}
				}
				let action = IOAction {
					to: to.ok_or_else(|| M::Error::missing_field("to"))?,
					if_exists: if_exists.unwrap_or_default(),
					sep: sep.unwrap_or_default(),
				};
				Ok(action)
			}
		}
		deserializer.deserialize_any(StringOrStruct(PhantomData))
	}
}
