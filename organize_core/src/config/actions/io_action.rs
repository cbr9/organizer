use std::{
	borrow::Cow,
	convert::Infallible,
	fmt,
	fs,
	io,
	io::{ErrorKind, Result},
	ops::Deref,
	path::{Path, PathBuf},
	result,
	str::FromStr,
};

use crate::{
	config::{ActionType, AsAction},
	path::{Expand, Update},
	string::{visit_placeholder_string, Placeholder},
};
use colored::Colorize;
use log::info;
use serde::{
	de,
	de::{Error, MapAccess, Visitor},
	export,
	export::PhantomData,
	Deserialize,
	Deserializer,
	Serialize,
};

#[derive(Debug, Clone, Deserialize, Eq, PartialEq)]
pub struct Sep(String);

impl Deref for Sep {
	type Target = String;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl Default for Sep {
	fn default() -> Self {
		Self(" ".into())
	}
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct IOAction {
	pub to: PathBuf,
	pub if_exists: ConflictOption,
	pub sep: Sep,
}

pub(super) struct Move;
pub(super) struct Rename;
pub(super) struct Copy;

impl AsAction<Move> for IOAction {
	fn act<'a>(&self, path: Cow<'a, Path>) -> Result<Cow<'a, Path>> {
		let to = Self::helper(&path, self, ActionType::Move)?;
		std::fs::rename(&path, &to)?;
		info!("({}) {} -> {}", ActionType::Move.to_string().bold(), path.display(), to.display());
		Ok(Cow::Owned(to))
	}
}

impl AsAction<Rename> for IOAction {
	fn act<'a>(&self, path: Cow<'a, Path>) -> Result<Cow<'a, Path>> {
		let to = IOAction::helper(&path, self, ActionType::Rename)?;
		fs::rename(&path, &to)?;
		info!("({}) {} -> {}", ActionType::Rename.to_string().bold(), path.display(), to.display());
		Ok(Cow::Owned(to))
	}
}

impl AsAction<Copy> for IOAction {
	fn act<'a>(&self, path: Cow<'a, Path>) -> Result<Cow<'a, Path>> {
		let to = IOAction::helper(&path, self, ActionType::Copy)?;
		std::fs::copy(&path, &to)?;
		info!("({}) {} -> {}", ActionType::Copy.to_string().bold(), path.display(), to.display());
		Ok(path)
	}
}

impl<'de> Deserialize<'de> for IOAction {
	fn deserialize<D>(deserializer: D) -> result::Result<Self, <D as Deserializer<'de>>::Error>
	where
		D: Deserializer<'de>,
	{
		struct StringOrStruct(PhantomData<fn() -> IOAction>);

		impl<'de> Visitor<'de> for StringOrStruct {
			type Value = IOAction;

			fn expecting(&self, formatter: &mut export::Formatter) -> fmt::Result {
				formatter.write_str("string or map")
			}

			fn visit_str<E>(self, value: &str) -> result::Result<Self::Value, E>
			where
				E: de::Error,
			{
				let string = visit_placeholder_string(value).map_err(|e| E::custom(e.to_string()))?;
				Ok(IOAction::from_str(string.as_str()).unwrap())
			}

			fn visit_map<M>(self, mut map: M) -> result::Result<Self::Value, M::Error>
			where
				M: MapAccess<'de>,
			{
				let mut action: IOAction = IOAction::default();
				while let Some((key, value)) = map.next_entry::<String, String>()? {
					match key.as_str() {
						"to" => {
							action.to = match visit_placeholder_string(&value) {
								Ok(str) => {
									let path = PathBuf::from(str).expand_vars().expand_user();
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

impl<T> From<T> for IOAction
where
	T: Into<PathBuf>,
{
	fn from(val: T) -> Self {
		Self {
			to: val.into(),
			if_exists: Default::default(),
			sep: Default::default(),
		}
	}
}

impl FromStr for IOAction {
	type Err = Infallible;

	fn from_str(s: &str) -> result::Result<Self, Self::Err> {
		let path = PathBuf::from(s);
		Ok(Self {
			to: path.expand_user().expand_vars(),
			if_exists: Default::default(),
			sep: Default::default(),
		})
	}
}

impl IOAction {
	fn helper<T>(path: T, action: &IOAction, kind: ActionType) -> Result<PathBuf>
	where
		T: AsRef<Path>,
	{
		debug_assert!([ActionType::Move, ActionType::Rename, ActionType::Copy].contains(&kind));

		let mut to: PathBuf = action.to.to_string_lossy().expand_placeholders(path.as_ref())?.deref().into();
		if kind == ActionType::Copy || kind == ActionType::Move {
			if !to.exists() {
				fs::create_dir_all(&to)?;
			}
			// to = to.canonicalize().unwrap();
			to.push(
				path.as_ref()
					.file_name()
					.ok_or_else(|| io::Error::new(ErrorKind::Other, "path has no filename"))?,
			);
		}

		if to.exists() {
			match to.update(&action.if_exists, &action.sep) {
				// FIXME: avoid the into_owned() call
				Ok(new_path) => to = new_path.into_owned(),
				Err(e) => return Err(e),
			}
		}
		Ok(to)
	}
}

/// Defines the options available to resolve a naming conflict,
/// i.e. how the application should proceed when a file exists
/// but it should move/rename/copy some file to that existing path
#[derive(Eq, PartialEq, Debug, Clone, Copy, Deserialize, Serialize)]
// for the config schema to keep these options as lowercase (i.e. the user doesn't have to
// write `if_exists: Rename`), and not need a #[allow(non_camel_case_types)] flag, serde
// provides the option to modify the fields are deserialize/serialize time
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum ConflictOption {
	Overwrite,
	Skip,
	Rename,
}

impl FromStr for ConflictOption {
	type Err = String;

	fn from_str(s: &str) -> result::Result<Self, Self::Err> {
		match s {
			"overwrite" => Ok(Self::Overwrite),
			"skip" => Ok(Self::Skip),
			"rename" => Ok(Self::Rename),
			_ => Err("invalid value".into()),
		}
	}
}

impl Default for ConflictOption {
	fn default() -> Self {
		ConflictOption::Rename
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		config::{ActionType, IOAction},
		utils::tests::project,
	};
	use serde_test::{assert_de_tokens, Token};

	#[test]
	fn deserialize_str() {
		let value = IOAction::from_str("$HOME").unwrap();
		assert_de_tokens(&value, &[Token::Str("$HOME")])
	}
	#[test]
	fn deserialize_map() {
		let mut value = IOAction::from_str("$HOME").unwrap();
		value.if_exists = ConflictOption::Rename;
		value.sep = Sep("-".into());
		assert_de_tokens(&value, &[
			Token::Map { len: Some(3) },
			Token::Str("to"),
			Token::Str("$HOME"),
			Token::Str("if_exists"),
			Token::Str("rename"),
			Token::Str("sep"),
			Token::Str("-"),
			Token::MapEnd,
		])
	}

	#[test]
	fn prepare_path_copy() {
		let original = project().join("tests").join("files").join("test1.txt");
		let target = project().join("tests").join("files").join("test_dir");
		let expected = target.join("test1 (1).txt");
		assert!(target.join(original.file_name().unwrap()).exists());
		assert!(!expected.exists());
		let action = IOAction::from(target);
		let new_path = IOAction::helper(&original, &action, ActionType::Copy).unwrap();
		assert_eq!(new_path, expected)
	}

	#[test]
	fn prepare_path_move() {
		let original = project().join("tests").join("files").join("test1.txt");
		let target = project().join("tests").join("files").join("test_dir");
		let expected = target.join("test1 (1).txt");
		assert!(target.join(original.file_name().unwrap()).exists());
		assert!(!expected.exists());
		let action = IOAction::from(target);
		let new_path = IOAction::helper(&original, &action, ActionType::Move).unwrap();
		assert_eq!(new_path, expected)
	}

	#[test]
	fn prepare_path_rename() {
		let original = project().join("tests").join("files").join("test1.txt");
		let target = original.with_file_name("test_dir").join(original.file_name().unwrap());
		let expected = target.with_file_name("test1 (1).txt");
		assert!(target.exists());
		assert!(!expected.exists());
		let action = IOAction::from(target);
		let new_path = IOAction::helper(&original, &action, ActionType::Rename).unwrap();
		assert_eq!(new_path, expected)
	}
}
