use std::{fmt, path::PathBuf, result, str::FromStr};

use crate::{config::Options, path::Expand};
use serde::{
	de,
	de::{MapAccess, Visitor},
	export,
	Deserialize,
	Deserializer,
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Folder {
	pub path: PathBuf,
	pub options: Option<Options>,
}

impl<'de> Deserialize<'de> for Folder {
	fn deserialize<D>(deserializer: D) -> result::Result<Self, <D as Deserializer<'de>>::Error>
	where
		D: Deserializer<'de>,
	{
		struct StringOrStruct;

		impl<'de> Visitor<'de> for StringOrStruct {
			type Value = Folder;

			fn expecting(&self, formatter: &mut export::Formatter) -> fmt::Result {
				formatter.write_str("string or map")
			}

			fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				Ok(Folder::from_str(v).unwrap())
			}

			fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
			where
				M: MapAccess<'de>,
			{
				let mut path: Option<String> = None;
				let mut options: Option<Options> = None;
				while let Some(key) = map.next_key::<String>()? {
					if key == "path" {
						path = Some(map.next_value()?);
					} else if key == "options" {
						options = Some(map.next_value()?);
					} else {
						return Err(serde::de::Error::custom(&format!("Invalid key: {}", key)));
					}
				}
				if path.is_none() {
					return Err(serde::de::Error::custom("Missing path"));
				}

				let mut folder = match Folder::from_str(path.unwrap().as_str()) {
					Ok(folder) => folder,
					Err(e) => return Err(serde::de::Error::custom(&format!("Path does not exist: {}", e))),
				};
				if let Some(options) = options {
					folder.options = Some(options);
				}
				Ok(folder)
			}
		}
		deserializer.deserialize_any(StringOrStruct)
	}
}

impl FromStr for Folder {
	type Err = std::io::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let path = PathBuf::from(s);
		match path.expand_user().expand_vars().canonicalize() {
			Ok(path) => Ok(Self { path, options: None }),
			Err(e) => Err(e),
		}
	}
}

pub type Folders = Vec<Folder>;
