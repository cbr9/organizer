use std::{
	ops::Deref,
	path::{Path, PathBuf},
	result,
	str::FromStr,
};
use std::convert::TryFrom;
use std::env::VarError;

use colored::Colorize;
use log::{debug, info};
use serde::{Deserialize, Serialize};

use crate::{
	data::config::actions::{ActionType, AsAction},
	path::{Expand, Update},
	string::Placeholder,
};

mod de;

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

impl Deref for Move {
	type Target = IOAction;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl Deref for Copy {
	type Target = IOAction;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl Deref for Rename {
	type Target = IOAction;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

macro_rules! io_action {
	($id:ty) => {
		impl AsAction for $id {
			fn act<T: Into<PathBuf>>(&self, path: T, simulate: bool) -> Option<PathBuf> {
				let path = path.into();
				let ty = self.ty();
				let to = self.prepare_path(&path, &ty)?;
				helper(ty, path, to, simulate)
			}
			fn ty(&self) -> ActionType {
				let name = stringify!($id).to_lowercase();
				ActionType::from_str(&name).expect(&format!("no variant associated with {}", name))
			}
		}
	}
}

io_action!(Move);
io_action!(Rename);
io_action!(Copy);

fn helper(ty: ActionType, path: PathBuf, to: PathBuf, simulate: bool) -> Option<PathBuf> {
	if !simulate {
		if let Some(parent) = to.parent() {
			if !parent.exists() {
				std::fs::create_dir_all(parent).map_err(|e| debug!("{}", e)).ok()?;
			}
		}
		match ty {
			ActionType::Copy => std::fs::copy(&path, &to).map(|_| ()),
			ActionType::Move | ActionType::Rename => std::fs::rename(&path, &to),
			_ => unreachable!(),
		}
			.map(|_| {
				info!("({}) {} -> {}", ty.to_string().bold(), path.display(), to.display());
				match ty {
					ActionType::Copy => path,
					ActionType::Move | ActionType::Rename => to,
					_ => unreachable!(),
				}
			})
			.map_err(|e| debug!("{}", e))
			.ok()
	} else {
		info!("(simulate {}) {} -> {}", ty.to_string().bold(), path.display(), to.display());
		Some(to)
	}
}

impl IOAction {
	fn prepare_path<T>(&self, path: T, kind: &ActionType) -> Option<PathBuf>
		where
			T: AsRef<Path>,
	{
		use ActionType::{Copy, Move, Rename};

		let path = path.as_ref();
		let mut to: PathBuf = self
			.to
			.to_string_lossy()
			.expand_placeholders(&path)
			.map_err(|e| debug!("{}", e))
			.ok()?
			.into();

		match kind {
			Copy | Move => {
				if to.to_string_lossy().ends_with("/") || to.is_dir() {
					to.push(path.file_name()?)
				}
			}
			Rename => {
				to = path.with_file_name(to);
			}
			_ => unreachable!(),
		}

		if to.exists() {
			to.update(&self.if_exists, &self.sep)
		} else {
			Some(to)
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
	use serde_test::{assert_de_tokens, Token};

	use crate::{data::config::actions::ActionType, utils::tests::project};

	use super::*;

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
		let new_path = action.prepare_path(&original, &ActionType::Copy).unwrap();
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
		let new_path = action.prepare_path(&original, &ActionType::Move).unwrap();
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
		let new_path = action.prepare_path(&original, &ActionType::Rename).unwrap();
		assert_eq!(new_path, expected)
	}
}
