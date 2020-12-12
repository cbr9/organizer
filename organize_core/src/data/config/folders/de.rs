use std::{fmt, result, str::FromStr};

use crate::{
	data::{config::folders::Folder, options::Options},
	utils::UnwrapOrDefaultOpt,
};
use serde::{
	de,
	de::{Error, MapAccess, Visitor},
	export, Deserialize, Deserializer,
};
use std::path::PathBuf;

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
				let mut path: Option<PathBuf> = None;
				let mut options: Option<Options> = None;
				while let Some(key) = map.next_key::<String>()? {
					match key.as_str() {
						"path" => {
							path = match path.is_none() {
								true => Some(Folder::from_str(&map.next_value::<String>()?).map(|f| f.path).map_err(M::Error::custom)?),
								false => return Err(M::Error::duplicate_field("path")),
							}
						}
						"options" => {
							options = match options.is_some() {
								true => return Err(M::Error::duplicate_field("options")),
								false => Some(map.next_value()?),
							};
						}
						_ => return Err(M::Error::unknown_field(key.as_str(), &["path", "options"])),
					}
				}
				let folder = Folder {
					path: path.ok_or_else(|| M::Error::missing_field("path"))?,
					options: options.unwrap_or_default_none(),
				};
				Ok(folder)
			}
		}
		deserializer.deserialize_any(StringOrStruct)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::data::options::recursive::Recursive;
	use crate::data::options::{
		apply::{wrapper::ApplyWrapper, Apply},
		Options,
	};
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
				Token::Bool(true),
				Token::Str("apply"),
				Token::UnitVariant {
					name: "Apply",
					variant: "all",
				},
				Token::Str("watch"),
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
				Token::Bool(true),
				Token::Str("apply"),
				Token::Str("all"),
				Token::Str("watch"),
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
				Token::Bool(true),
				Token::Str("apply"),
				Token::Str("all"),
				Token::Str("watch"),
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
				Token::Bool(true),
				Token::Str("apply"),
				Token::Str("all"),
				Token::Str("watch"),
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
		value.options = Options {
			recursive: Recursive {
				enabled: Some(true),
				depth: None,
			},
			watch: Some(true),
			ignored_dirs: None,
			hidden_files: None,
			r#match: None,
			apply: ApplyWrapper::from(Apply::All),
		};
		assert_de_tokens(
			&value,
			&[
				Token::Map { len: Some(2) },
				Token::Str("path"),
				Token::Str("$HOME"),
				Token::Str("options"),
				Token::Map { len: Some(3) },
				Token::Str("recursive"),
				Token::Bool(true),
				Token::Str("apply"),
				Token::Str("all"),
				Token::Str("watch"),
				Token::Bool(true),
				Token::MapEnd,
				Token::MapEnd,
			],
		)
	}
}
