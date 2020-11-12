use std::{fmt, path::PathBuf, result, str::FromStr};

use crate::{config::Options, path::Expand};
use serde::{
	de,
	de::{Error, MapAccess, Visitor},
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
				Folder::from_str(v).map_err(E::custom)
			}

			fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
			where
				M: MapAccess<'de>,
			{
				let mut folder = Folder {
					path: Default::default(),
					options: None,
				};
				while let Some(key) = map.next_key::<String>()? {
					match key.as_str() {
						"path" => {
							folder = match Folder::from_str(&map.next_value::<String>()?) {
								Ok(mut new_folder) => {
									new_folder.options = folder.options;
									new_folder
								}
								Err(e) => return Err(M::Error::custom(e)),
							}
						}
						"options" => {
							folder.options = Some(map.next_value()?);
						}
						_ => return Err(M::Error::custom("unknown field")),
					}
				}
				if folder.path == PathBuf::default() {
					return Err(serde::de::Error::custom("missing path"));
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::config::{Apply, ApplyWrapper};
	use serde_test::{assert_de_tokens, assert_de_tokens_error, Token};

	#[test]
	fn deserialize_str() {
		let value = Folder::from_str("$HOME").unwrap();
		assert_de_tokens(&value, &[Token::Str("$HOME")])
	}
	#[test]
	fn deserialize_map_invalid() {
		assert_de_tokens_error::<Folder>(
			&[
				Token::Map { len: Some(1) },
				Token::Str("options"),
				Token::Map { len: Some(3) },
				Token::Str("recursive"),
				Token::Some,
				Token::Bool(true),
				Token::Str("apply"),
				Token::Some,
				Token::Str("all"),
				Token::Str("watch"),
				Token::Some,
				Token::Bool(true),
				Token::MapEnd,
				Token::MapEnd,
			],
			"missing path",
		)
	}
	#[test]
	fn deserialize_map_valid() {
		let mut value = Folder::from_str("$HOME").unwrap();
		value.options = Some(Options {
			recursive: Some(true),
			watch: Some(true),
			ignore: None,
			hidden_files: None,
			r#match: None, // TODO: create issue in serde_test about raw identifiers not working properly
			apply: Some(ApplyWrapper::from(Apply::All)),
		});
		assert_de_tokens(&value, &[
			Token::Map { len: Some(2) },
			Token::Str("path"),
			Token::Str("$HOME"),
			Token::Str("options"),
			Token::Map { len: Some(3) },
			Token::Str("recursive"),
			Token::Some,
			Token::Bool(true),
			Token::Str("apply"),
			Token::Some,
			Token::Str("all"),
			Token::Str("watch"),
			Token::Some,
			Token::Bool(true),
			Token::MapEnd,
			Token::MapEnd,
		])
	}
}
