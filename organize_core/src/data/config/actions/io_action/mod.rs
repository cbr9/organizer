use std::{
	convert::TryFrom,
	path::{Path, PathBuf},
	result,
	str::FromStr,
};

use log::{error, info};
use serde::{Deserialize, Serialize};

use crate::{
	data::config::actions::{Act, ActionType, AsAction, Simulate},
	path::{Expand, ResolveConflict},
	simulation::Simulation,
	string::Placeholder,
	utils::UnwrapRef,
};
use anyhow::{Context, Result};

use regex::Regex;
use serde::de::Error;
use std::sync::{Arc, Mutex, MutexGuard};

mod de;

#[derive(Debug, Clone, Eq, PartialEq, Default)]
struct Inner {
	pub to: PathBuf,
	pub if_exists: ConflictOption,
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
			fn process<T: Into<PathBuf>>(&self, path: T, simulation: Option<&Arc<Mutex<Simulation>>>) -> Option<PathBuf> {
				let path = path.into();
				let ty = self.ty();
				let to = self.0.prepare_path(&path, &ty, simulation);
				if to.is_none() {
					if self.0.if_exists == ConflictOption::Delete {
						match simulation {
							None => {
								if let Err(e) = std::fs::remove_file(&path).with_context(|| format!("could not delete {}", path.display())) {
									error!("{:?}", e);
								}
							}
							Some(simulation) => {
								let mut guard = simulation.lock().unwrap();
								guard.remove_file(&path);
							}
						}
					}
					return None;
				}
				if simulation.is_none() {
					match to.unwrap_ref().parent() {
						Some(parent) => {
							if !parent.exists() {
								if let Err(e) = std::fs::create_dir_all(parent)
									.with_context(|| format!("could not create parent directory for {}", to.unwrap_ref().display()))
								{
									error!("{:?}", e);
									return None;
								}
							}
						}
						None => {
							error!("{} has an invalid parent", to.unwrap().display());
							return None;
						}
					}
				}
				match simulation {
					Some(simulation) => {
						let mut guard = simulation.lock().unwrap();
						let parent = to.unwrap_ref().parent()?;
						if parent.exists() {
							guard
								.watch_folder(parent)
								.map_err(|e| eprintln!("Error: {} ({})", e, parent.display()))
								.ok()?;
						}
						match self.simulate(&path, Some(to.unwrap_ref()), guard) {
							Ok(new_path) => {
								info!("(simulate {}) {} -> {}", ty.to_string(), path.display(), to.unwrap().display());
								new_path
							}
							Err(e) => {
								error!("{:?}", e);
								None
							}
						}
					}
					None => match self.act(&path, Some(to.unwrap_ref())) {
						Ok(new_path) => {
							info!("({}) {} -> {}", ty.to_string(), path.display(), to.unwrap().display());
							new_path
						}
						Err(e) => {
							error!("{:?}", e);
							None
						}
					},
				}
			}

			fn ty(&self) -> ActionType {
				let name = stringify!($id).to_lowercase();
				ActionType::from_str(&name).expect(&format!("no variant associated with {}", name))
			}
		}
	};
}

as_action!(Move);
as_action!(Rename);
as_action!(Copy);
as_action!(Hardlink);
as_action!(Symlink);

impl Simulate for Move {
	fn simulate<T, U>(&self, from: T, to: Option<U>, mut guard: MutexGuard<Simulation>) -> Result<Option<PathBuf>>
	where
		Self: Sized,
		T: AsRef<Path> + Into<PathBuf>,
		U: AsRef<Path> + Into<PathBuf>,
	{
		let to = to.unwrap().into();
		guard.remove_file(from);
		guard.insert_file(&to);
		Ok(Some(to))
	}
}

impl Act for Move {
	fn act<T, P>(&self, from: T, to: Option<P>) -> Result<Option<PathBuf>>
	where
		T: AsRef<Path> + Into<PathBuf>,
		P: AsRef<Path> + Into<PathBuf>,
	{
		std::fs::rename(&from, to.unwrap_ref())
			.with_context(|| format!("could not move ({} -> {})", from.as_ref().display(), to.unwrap_ref().as_ref().display()))
			.map(|_| Some(to.unwrap().into()))
	}
}

impl Simulate for Copy {
	fn simulate<T, U>(&self, from: T, to: Option<U>, mut guard: MutexGuard<Simulation>) -> Result<Option<PathBuf>>
	where
		Self: Sized,
		T: AsRef<Path> + Into<PathBuf>,
		U: AsRef<Path> + Into<PathBuf>,
	{
		guard.insert_file(to.unwrap());
		Ok(Some(from.into()))
	}
}

impl Act for Copy {
	fn act<T, P>(&self, from: T, to: Option<P>) -> Result<Option<PathBuf>>
	where
		T: AsRef<Path> + Into<PathBuf>,
		P: AsRef<Path> + Into<PathBuf>,
	{
		std::fs::copy(&from, to.unwrap_ref())
			.with_context(|| format!("could not copy ({} -> {})", from.as_ref().display(), to.unwrap().as_ref().display()))
			.map(|_| Some(from.into()))
	}
}

impl Simulate for Rename {
	fn simulate<T, U>(&self, from: T, to: Option<U>, mut guard: MutexGuard<Simulation>) -> Result<Option<PathBuf>>
	where
		Self: Sized,
		T: AsRef<Path> + Into<PathBuf>,
		U: AsRef<Path> + Into<PathBuf>,
	{
		let to = to.unwrap().into();
		guard.remove_file(from);
		guard.insert_file(&to);
		Ok(Some(to))
	}
}

impl Act for Rename {
	fn act<T, P>(&self, from: T, to: Option<P>) -> Result<Option<PathBuf>>
	where
		T: AsRef<Path> + Into<PathBuf>,
		P: AsRef<Path> + Into<PathBuf>,
	{
		std::fs::rename(&from, to.unwrap_ref())
			.with_context(|| format!("could not rename ({} -> {})", from.as_ref().display(), to.unwrap_ref().as_ref().display()))
			.map(|_| Some(to.unwrap().into()))
	}
}

impl Simulate for Hardlink {
	fn simulate<T, U>(&self, from: T, to: Option<U>, mut guard: MutexGuard<Simulation>) -> Result<Option<PathBuf>>
	where
		Self: Sized,
		T: AsRef<Path> + Into<PathBuf>,
		U: AsRef<Path> + Into<PathBuf>,
	{
		guard.insert_file(to.unwrap());
		Ok(Some(from.into()))
	}
}

impl Act for Hardlink {
	fn act<T, P>(&self, from: T, to: Option<P>) -> Result<Option<PathBuf>>
	where
		T: AsRef<Path> + Into<PathBuf>,
		P: AsRef<Path> + Into<PathBuf>,
	{
		std::fs::hard_link(&from, to.unwrap_ref())
			.with_context(|| {
				format!(
					"could not create hardlink ({} -> {})",
					from.as_ref().display(),
					to.unwrap_ref().as_ref().display()
				)
			})
			.map(|_| Some(from.into()))
	}
}

impl Simulate for Symlink {
	fn simulate<T, U>(&self, from: T, to: Option<U>, mut guard: MutexGuard<Simulation>) -> Result<Option<PathBuf>>
	where
		Self: Sized,
		T: AsRef<Path> + Into<PathBuf>,
		U: AsRef<Path> + Into<PathBuf>,
	{
		guard.insert_file(to.unwrap());
		Ok(Some(from.into()))
	}
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
impl Act for Symlink {
	fn act<T, P>(&self, from: T, to: Option<P>) -> Result<Option<PathBuf>>
	where
		T: AsRef<Path> + Into<PathBuf>,
		P: AsRef<Path> + Into<PathBuf>,
	{
		std::os::unix::fs::symlink(&from, to.unwrap_ref())
			.with_context(|| {
				format!(
					"could not create symlink ({} -> {})",
					from.as_ref().display(),
					to.unwrap_ref().as_ref().display()
				)
			})
			.map(|_| Some(from.into()))
	}
}

impl Inner {
	fn prepare_path<T>(&self, path: T, kind: &ActionType, simulation: Option<&Arc<Mutex<Simulation>>>) -> Option<PathBuf>
	where
		T: AsRef<Path>,
	{
		use ActionType::{Copy, Hardlink, Move, Rename, Symlink};

		let path = path.as_ref();
		let mut to = match self.to.to_string_lossy().expand_placeholders(path) {
			Ok(str) => PathBuf::from(str),
			Err(e) => {
				error!("{:?}", e);
				return None;
			}
		};

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
					to.resolve_naming_conflict(&self.if_exists, None)
				} else {
					Some(to)
				}
			}
			Some(sim) => {
				let guard = sim.lock().unwrap();
				if guard.files.contains(&to) {
					to.resolve_naming_conflict(&self.if_exists, Some(guard))
				} else {
					Some(to)
				}
			}
		}
	}
}

impl TryFrom<PathBuf> for Inner {
	type Error = anyhow::Error;

	fn try_from(value: PathBuf) -> result::Result<Self, Self::Error> {
		let action = Self {
			to: value.expand_user()?.expand_vars()?,
			if_exists: Default::default(),
		};
		Ok(action)
	}
}

impl FromStr for Inner {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> result::Result<Self, Self::Err> {
		Self::try_from(PathBuf::from(s))
	}
}

/// Defines the options available to resolve a naming conflict,
/// i.e. how the application should proceed when a file exists
/// but it should move/rename/copy some file to that existing path
#[derive(Eq, PartialEq, Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum ConflictOption {
	Overwrite,
	Skip,
	Rename { counter_separator: String },
	Delete,
}

impl FromStr for ConflictOption {
	type Err = serde::de::value::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let variant = match s {
			"delete" => Self::Delete,
			"overwrite" => Self::Overwrite,
			"skip" => Self::Skip,
			"rename" => Self::default(),
			other => {
				let re = Regex::new("rename with \"(?P<counter_separator>.*)\"").unwrap();
				let captures = re.captures(other).ok_or_else(|| {
					Self::Err::unknown_variant(other, &["skip", "delete", "overwrite", "rename", "rename with \"<counter_separator>\""])
				})?;
				let counter_separator = captures.name("counter_separator").unwrap();
				Self::Rename {
					counter_separator: counter_separator.as_str().into(),
				}
			}
		};
		Ok(variant)
	}
}

impl Default for ConflictOption {
	fn default() -> Self {
		ConflictOption::Rename {
			counter_separator: " ".to_string(),
		}
	}
}

#[cfg(test)]
mod tests {

	use crate::{
		data::config::actions::ActionType,
		utils::tests::{project, AndWait, TEST_FILES_DIRECTORY, TEST_FILES_SUBDIRECTORY},
	};
	use std::fs::File;

	use super::*;
	use anyhow::Result;
	use std::ops::Deref;

	#[test]
	fn conflict_option_from_str() -> Result<()> {
		assert_eq!(ConflictOption::from_str("skip")?, ConflictOption::Skip);
		assert_eq!(ConflictOption::from_str("delete")?, ConflictOption::Delete);
		assert_eq!(ConflictOption::from_str("overwrite")?, ConflictOption::Overwrite);
		assert_eq!(ConflictOption::from_str("rename")?, ConflictOption::default());
		assert_eq!(ConflictOption::from_str("rename with \" - \"")?, ConflictOption::Rename {
			counter_separator: " - ".to_string()
		});
		assert!(ConflictOption::from_str("rename with").is_err());
		assert!(ConflictOption::from_str("rename with \"").is_err());
		assert!(ConflictOption::from_str("rename with \" - ").is_err());
		Ok(())
	}

	#[test]
	fn prepare_path_copy_rename_exists() -> Result<()> {
		let filename = "test.txt";
		let from = TEST_FILES_DIRECTORY.join(filename);
		let to = TEST_FILES_SUBDIRECTORY.deref();
		File::create_and_wait(&from)?;
		File::create_and_wait(to.join(filename))?;
		let expected = to.join("test (1).txt");
		let action = Inner::try_from(to.clone())?;
		let new_path = action.prepare_path(&from, &ActionType::Copy, None).unwrap();
		File::remove_and_wait(from)?;
		File::remove_and_wait(to.join(filename))?;
		assert_eq!(new_path, expected);
		Ok(())
	}
	#[test]
	fn prepare_path_copy_simulation() -> Result<()> {
		let simulation = Simulation::new()?;
		let filename = "prepare_path_copy_simulation.txt";
		let from = TEST_FILES_DIRECTORY.join(filename);
		let to = TEST_FILES_SUBDIRECTORY.deref();
		{
			let mut guard = simulation.lock().unwrap();
			guard.insert_file(&from);
			guard.insert_file(to.join(filename));
		}
		let expected = to.join("prepare_path_copy_simulation (1).txt");
		let action = Inner::try_from(to.clone())?;
		let new_path = action.prepare_path(&from, &ActionType::Copy, Some(&simulation)).unwrap();
		assert_eq!(new_path, expected);
		Ok(())
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
