mod de;

use std::{
	ops::Deref,
	path::{Path, PathBuf},
	result,
	str::FromStr,
};

use crate::{
	data::config::actions::{ActionType, AsAction},
	path::{Expand, Update},
	string::Placeholder,
};
use colored::Colorize;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::env::VarError;

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct IOAction {
	pub to: PathBuf,
	pub if_exists: ConflictOption,
	pub sep: Sep,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Move(IOAction);
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Rename(IOAction);
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Copy(IOAction);

impl AsAction for Move {
	fn act<T: Into<PathBuf>>(&self, path: T, simulate: bool) -> Option<PathBuf> {
		let path = path.into();
		let to = IOAction::helper(&path, &self.0, ActionType::Move)?;
		if !simulate {
			std::fs::rename(&path, &to)
				.map(|_| {
					info!("({}) {} -> {}", ActionType::Move.to_string().bold(), path.display(), to.display());
					to
				})
				.map_err(|e| debug!("{}", e))
				.ok()
		} else {
			info!(
				"(simulate {}) {} -> {}",
				ActionType::Move.to_string().bold(),
				path.display(),
				to.display()
			);
			Some(to)
		}
	}
}

impl AsAction for Rename {
	fn act<T: Into<PathBuf>>(&self, path: T, simulate: bool) -> Option<PathBuf> {
		let path = path.into();
		let to = IOAction::helper(&path, &self.0, ActionType::Rename)?;
		if !simulate {
			std::fs::rename(&path, &to)
				.map(|_| {
					info!("({}) {} -> {}", ActionType::Rename.to_string().bold(), path.display(), to.display());
					to
				})
				.map_err(|e| debug!("{}", e))
				.ok()
		} else {
			info!(
				"(simulate {}) {} -> {}",
				ActionType::Rename.to_string().bold(),
				path.display(),
				to.display()
			);
			Some(to)
		}
	}
}

impl AsAction for Copy {
	fn act<T: Into<PathBuf>>(&self, path: T, simulate: bool) -> Option<PathBuf> {
		let path = path.into();
		let to = IOAction::helper(&path, &self.0, ActionType::Copy)?;
		if !simulate {
			std::fs::copy(&path, &to)
				.map(|_| {
					info!("({}) {} -> {}", ActionType::Copy.to_string().bold(), path.display(), to.display());
					to
				})
				.map_err(|e| debug!("{}", e))
				.ok()
		} else {
			info!(
				"(simulate {}) {} -> {}",
				ActionType::Copy.to_string().bold(),
				path.display(),
				to.display()
			);
			Some(path)
		}
	}
}

impl TryFrom<PathBuf> for IOAction {
	type Error = VarError;

	fn try_from(value: PathBuf) -> result::Result<Self, Self::Error> {
		let action = Self {
			to: value.expand_user()?.expand_vars()?,
			if_exists: Default::default(),
			sep: Default::default(),
		};
		Ok(action)
	}
}

impl FromStr for IOAction {
	type Err = VarError;

	fn from_str(s: &str) -> result::Result<Self, Self::Err> {
		Self::try_from(PathBuf::from(s))
	}
}

impl IOAction {
	fn helper<T>(path: T, action: &IOAction, kind: ActionType) -> Option<PathBuf>
	where
		T: AsRef<Path>,
	{
		use ActionType::{Copy, Move};

		let mut to: PathBuf = action.to.to_string_lossy().expand_placeholders(&path).ok()?.into();
		match kind {
			Copy | Move => to.push(path.as_ref().file_name()?),
			_ => {}
		}
		if to.exists() {
			to.update(&action.if_exists, &action.sep)
		} else {
			Some(to)
		}
	}
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq)]
pub struct Sep(pub String);

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

/// Defines the options available to resolve a naming conflict,
/// i.e. how the application should proceed when a file exists
/// but it should move/rename/copy some file to that existing path
#[derive(Eq, PartialEq, Debug, Clone, Copy, Deserialize, Serialize, EnumString)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum ConflictOption {
	Overwrite,
	Skip,
	Rename,
}

impl Default for ConflictOption {
	fn default() -> Self {
		ConflictOption::Rename
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{data::config::actions::ActionType, utils::tests::project};
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
		assert_de_tokens(
			&value,
			&[
				Token::Map { len: Some(3) },
				Token::Str("to"),
				Token::Str("$HOME"),
				Token::Str("if_exists"),
				Token::Str("rename"),
				Token::Str("sep"),
				Token::Str("-"),
				Token::MapEnd,
			],
		)
	}

	#[test]
	fn prepare_path_copy() {
		let original = project().join("tests").join("files").join("test1.txt");
		let target = project().join("tests").join("files").join("test_dir");
		let expected = target.join("test1 (1).txt");
		assert!(target.join(original.file_name().unwrap()).exists());
		assert!(!expected.exists());
		let action = IOAction::try_from(target).unwrap();
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
		let action = IOAction::try_from(target).unwrap();
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
		let action = IOAction::try_from(target).unwrap();
		let new_path = IOAction::helper(&original, &action, ActionType::Rename).unwrap();
		assert_eq!(new_path, expected)
	}
}
