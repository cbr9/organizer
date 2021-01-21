use std::convert::TryFrom;
use std::env::VarError;
use std::{
	ops::Deref,
	path::{Path, PathBuf},
	result,
	str::FromStr,
};

use log::{error, info};
use serde::{Deserialize, Serialize};

use crate::{
	data::config::actions::{ActionType, AsAction},
	path::{Expand, Update},
	simulation::Simulation,
	string::Placeholder,
};
use std::sync::{Arc, Mutex};
use log::warn;

mod de;

#[derive(Debug, Clone, Eq, PartialEq, Default)]
struct Inner {
	pub to: PathBuf,
	pub if_exists: ConflictOption,
	pub sep: Sep,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Move(Inner);

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Rename(Inner);

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Copy(Inner);

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Hardlink(Inner);

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Symlink(Inner);

macro_rules! as_action {
	($id:ty) => {
		impl AsAction for $id {
			fn act<T: Into<PathBuf>>(&self, path: T) -> Option<PathBuf> {
				let path = path.into();
				let ty = self.ty();
				let to = self.0.prepare_path(&path, &ty, None);
				if to.is_none() {
					if self.0.if_exists == ConflictOption::Delete {
						std::fs::remove_file(&path).map_err(|e| {
							warn!("could not delete {} ({})", path.display(), e)
						}).ok()?;
					}
					return None
				}
				act(ty, path, to.unwrap())
			}
			fn ty(&self) -> ActionType {
				let name = stringify!($id).to_lowercase();
				ActionType::from_str(&name).expect(&format!("no variant associated with {}", name))
			}
			fn simulate<T: Into<PathBuf>>(&self, path: T, simulation: &Arc<Mutex<Simulation>>) -> Option<PathBuf> {
				let path = path.into();
				let ty = self.ty();
				let to = self.0.prepare_path(&path, &ty, Some(simulation));
				if to.is_none() {
					if self.0.if_exists == ConflictOption::Delete {
						let mut guard = simulation.lock().unwrap();
						guard.files.remove(&path);
					}
					return None
				}
				simulate(ty, path, to.unwrap(), simulation)
			}
		}
	}
}

as_action!(Move);
as_action!(Rename);
as_action!(Copy);
as_action!(Hardlink);
as_action!(Symlink);

fn simulate(ty: ActionType, from: PathBuf, to: PathBuf, simulation: &Arc<Mutex<Simulation>>) -> Option<PathBuf> {
	use ActionType::{Copy, Hardlink, Move, Rename, Symlink};
	let mut ptr = simulation.lock().unwrap();
	ptr.watch_folder(to.parent()?).map_err(|e| eprintln!("{}", e)).ok()?;
	info!("(simulate {}) {} -> {}", ty.to_string(), from.display(), to.display());
	match ty {
		Copy | Hardlink | Symlink => {
			ptr.insert_file(to);
			Some(from)
		}
		Move | Rename => {
			ptr.remove_file(from);
			ptr.insert_file(to.clone());
			Some(to)
		}
		_ => unreachable!(),
	}
}

fn act(ty: ActionType, from: PathBuf, to: PathBuf) -> Option<PathBuf> {
	use ActionType::{Copy, Hardlink, Move, Rename, Symlink};
	if let Some(parent) = to.parent() {
		if !parent.exists() {
			std::fs::create_dir_all(parent).map_err(|e| error!("{}", e)).ok()?;
		}
	}
	match ty {
		Copy => std::fs::copy(&from, &to).map(|_| ()),
		Move | Rename => std::fs::rename(&from, &to),
		Hardlink => std::fs::hard_link(&from, &to),
		Symlink => std::os::unix::fs::symlink(&from, &to),
		_ => unreachable!(),
	}
	.map(|_| {
		info!("({}) {} -> {}", ty.to_string(), from.display(), to.display());
		match ty {
			Copy | Hardlink | Symlink => from,
			Move | Rename => to,
			_ => unreachable!(),
		}
	})
	.map_err(|e| error!("{}", e))
	.ok()
}

impl Inner {
	fn prepare_path<T>(&self, path: T, kind: &ActionType, simulation: Option<&Arc<Mutex<Simulation>>>) -> Option<PathBuf>
	where
		T: AsRef<Path>,
	{
		use ActionType::{Copy, Hardlink, Move, Rename, Symlink};

		let path = path.as_ref();
		let mut to: PathBuf = self
			.to
			.to_string_lossy()
			.expand_placeholders(&path)
			.map_err(|e| error!("{}", e))
			.ok()?
			.into();

		match kind {
			Copy | Move | Hardlink | Symlink => {
				if to.to_string_lossy().ends_with('/') || to.is_dir() {
					to.push(path.file_name()?)
				}
			}
			Rename => {
				to = path.with_file_name(to);
			}
			_ => unreachable!(),
		}

		match simulation {
			None => {
				if to.exists() {
					to.update(&self.if_exists, &self.sep, None)
				} else {
					Some(to)
				}
			}
			Some(sim) => {
				let guard = sim.lock().unwrap();
				if guard.files.contains(&to) {
					to.update(&self.if_exists, &self.sep, Some(sim))
				} else {
					Some(to)
				}
			}
		}
	}
}

impl TryFrom<PathBuf> for Inner {
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

impl FromStr for Inner {
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
	Delete
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
		let value = Inner::from_str("$HOME").unwrap();
		assert_de_tokens(&value, &[Token::Str("$HOME")])
	}

	#[test]
	fn deserialize_map() {
		let mut value = Inner::from_str("$HOME").unwrap();
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
		let action = Inner::try_from(target).unwrap();
		let new_path = action.prepare_path(&original, &ActionType::Copy, None).unwrap();
		assert_eq!(new_path, expected)
	}

	#[test]
	fn prepare_path_move() {
		let original = project().join("tests").join("files").join("test1.txt");
		let target = project().join("tests").join("files").join("test_dir");
		let expected = target.join("test1 (1).txt");
		assert!(target.join(original.file_name().unwrap()).exists());
		assert!(!expected.exists());
		let action = Inner::try_from(target).unwrap();
		let new_path = action.prepare_path(&original, &ActionType::Move, None).unwrap();
		assert_eq!(new_path, expected)
	}

	#[test]
	fn prepare_path_rename() {
		let original = project().join("tests").join("files").join("test1.txt");
		let target = original.with_file_name("test_dir").join(original.file_name().unwrap());
		let expected = target.with_file_name("test1 (1).txt");
		assert!(target.exists());
		assert!(!expected.exists());
		let action = Inner::try_from(target).unwrap();
		let new_path = action.prepare_path(&original, &ActionType::Rename, None).unwrap();
		assert_eq!(new_path, expected)
	}
}
