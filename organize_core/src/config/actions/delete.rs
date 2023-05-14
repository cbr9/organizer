use std::path::{Path, PathBuf};

use crate::config::actions::{Act, ActionType, AsAction};
use anyhow::{Context, Result};
use derive_more::Deref;
use log::{error, info};
use serde::Deserialize;
use std::str::FromStr;

#[derive(Debug, Clone, Deref, Deserialize, Default, PartialEq, Eq)]
pub struct Delete(bool);

#[derive(Debug, Clone, Deref, Deserialize, Default, Eq, PartialEq)]
pub struct Trash(bool);

macro_rules! as_action {
	($id:ty) => {
		impl AsAction for $id {
			fn process<T: Into<PathBuf> + AsRef<Path>>(&self, path: T) -> Option<PathBuf> {
				let path = path.into();
				let to: Option<T> = None;
				if **self {
					match self.act(&path, to) {
						Ok(new_path) => {
							info!("({}) {}", self.ty().to_string(), path.display());
							new_path
						}
						Err(e) => {
							error!("{:?}", e);
							None
						}
					}
				} else {
					Some(path)
				}
			}

			fn ty(&self) -> ActionType {
				let name = stringify!($id).to_lowercase();
				ActionType::from_str(&name).expect(&format!("no variant associated with {}", name))
			}
		}
	};
}

as_action!(Delete);
as_action!(Trash);

impl Act for Delete {
	fn act<T, P>(&self, from: T, _to: Option<P>) -> Result<Option<PathBuf>>
	where
		T: AsRef<Path> + Into<PathBuf>,
		P: AsRef<Path> + Into<PathBuf>,
	{
		if **self {
			std::fs::remove_file(&from)
				.with_context(|| format!("could not delete {}", from.as_ref().display()))
				.map(|_| None)
		} else {
			Ok(Some(from.into()))
		}
	}
}

impl Trash {
	fn dir() -> Result<PathBuf> {
		let dir = dirs_next::data_local_dir().unwrap().join("organize").join(".trash");
		std::fs::create_dir_all(&dir)
			.with_context(|| format!("Could not create trash directory at {}", &dir.display()))
			.map(|_| dir)
	}
}

impl Act for Trash {
	fn act<T, P>(&self, from: T, _to: Option<P>) -> Result<Option<PathBuf>>
	where
		T: AsRef<Path> + Into<PathBuf>,
		P: AsRef<Path> + Into<PathBuf>,
	{
		if self.0 {
			let to = Self::dir()?.join(from.as_ref().file_name().unwrap());
			let from = from.as_ref();
			std::fs::copy(&from, &to).with_context(|| format!("Could not copy file ({} -> {})", from.display(), to.display()))?;
			std::fs::remove_file(&from)
				.with_context(|| format!("could not move ({} -> {})", from.display(), to.display()))
				.map(|_| None)
		} else {
			Ok(Some(from.into()))
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use tempfile;

	#[test]
	fn test_delete_true() {
		let tmp_dir = tempfile::tempdir().expect("Couldn't create temporary directory");
		let tmp_path = tmp_dir.path().to_owned();
		let tmp_file = tmp_path.join("delete_me.txt");
		let action = Delete(true);

		std::fs::write(&tmp_file, "").expect("Could create target file");
		assert!(tmp_file.exists());

		action
			.act::<&Path, &Path>(&tmp_file, None)
			.expect("Could not delete target file");
		assert!(!tmp_file.exists());
	}

	#[test]
	fn test_delete_false() {
		let tmp_dir = tempfile::tempdir().expect("Couldn't create temporary directory");
		let tmp_path = tmp_dir.path().to_owned();
		let tmp_file = tmp_path.join("delete_me.txt");
		let action = Delete(false);

		std::fs::write(&tmp_file, "").expect("Could create target file");
		assert!(tmp_file.exists());

		action
			.act::<&Path, &Path>(&tmp_file, None)
			.expect("Could not `delete` target file");
		assert!(tmp_file.exists());
	}

	#[test]
	fn test_trash_true() {
		let tmp_dir = tempfile::tempdir().expect("Couldn't create temporary directory");
		let tmp_path = tmp_dir.path().to_owned();
		let tmp_file = tmp_path.join("trash_me.txt");
		let action = Trash(true);

		std::fs::write(&tmp_file, "").expect("Could create target file");
		assert!(tmp_file.exists());

		let new_path = action
			.act::<&Path, &Path>(&tmp_file, None)
			.expect("Could not delete target file");
		dbg!(new_path);
		assert!(!tmp_file.exists());
	}

	#[test]
	fn test_trash_false() {
		let tmp_dir = tempfile::tempdir().expect("Couldn't create temporary directory");
		let tmp_path = tmp_dir.path().to_owned();
		let tmp_file = tmp_path.join("trash_me.txt");
		let action = Trash(false);

		std::fs::write(&tmp_file, "").expect("Could create target file");
		assert!(tmp_file.exists());

		action
			.act::<&Path, &Path>(&tmp_file, None)
			.expect("Could not `delete` target file");
		assert!(tmp_file.exists());
	}
}
