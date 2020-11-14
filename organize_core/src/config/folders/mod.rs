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
	fn deserialize<D>(deserializer: D) -> result::Result<Self, D::Error>
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
					path: PathBuf::default(),
					options: None,
				};
				while let Some(key) = map.next_key::<String>()? {
					match key.as_str() {
						"path" => {
							folder = match folder.path == PathBuf::default() {
								false => return Err(M::Error::duplicate_field("path")),
								true => match Folder::from_str(&map.next_value::<String>()?) {
									Ok(mut new_folder) => {
										new_folder.options = folder.options;
										new_folder
									}
									Err(e) => return Err(M::Error::custom(e)),
								},
							}
						}
						"options" => {
							folder.options = match folder.options.is_some() {
								true => return Err(M::Error::duplicate_field("options")),
								false => Some(map.next_value()?),
							};
						}
						_ => return Err(M::Error::unknown_field(key.as_str(), &["path", "options"])),
					}
				}
				if folder.path == PathBuf::default() {
					return Err(M::Error::missing_field("path"));
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
	use serde::de::{value::Error, Error as ErrorTrait};
	use serde_test::{assert_de_tokens, assert_de_tokens_error, Token};

	#[test]
	fn deserialize_str() {
		let value = Folder::from_str("$HOME").unwrap();
		assert_de_tokens(&value, &[Token::Str("$HOME")])
	}
	#[test]
	fn deserialize_map_missing_path() {
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
			&Error::missing_field("path").to_string(),
		)
	}
	#[test]
	fn deserialize_map_duplicate_options() {
		assert_de_tokens_error::<Folder>(
			&[
				Token::Map { len: Some(3) },
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
				Token::Str("options"),
				Token::MapEnd,
			],
			&Error::duplicate_field("options").to_string(),
		)
	}
	#[test]
	fn deserialize_map_duplicate_path() {
		assert_de_tokens_error::<Folder>(
			&[
				Token::Map { len: Some(3) },
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
				Token::Str("path"),
				Token::MapEnd,
			],
			&format!("{}", &Error::duplicate_field("path")),
		)
	}
	#[test]
	fn deserialize_map_unknown_field() {
		assert_de_tokens_error::<Folder>(
			&[
				Token::Map { len: Some(3) },
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
				Token::Str("unknown"),
				Token::MapEnd,
			],
			&Error::unknown_field("unknown", &["path", "options"]).to_string(),
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
			r#match: None,
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
