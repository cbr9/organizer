use std::{
	ops::Deref,
	path::{Path, PathBuf},
};

use crate::data::actions::{Act, ActionType, AsAction};
use anyhow::{Context, Result};
use log::{error, info};
use serde::Deserialize;
use std::str::FromStr;

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
pub struct Delete(bool);

#[derive(Debug, Clone, Deserialize, Default, Eq, PartialEq)]
pub struct Trash(bool);

impl Deref for Delete {
	type Target = bool;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

macro_rules! as_action {
	($id:ty) => {
		impl AsAction for $id {
			fn process<T: Into<PathBuf> + AsRef<Path>>(&self, path: T) -> Option<PathBuf> {
				let path = path.into();
				let to: Option<T> = None;
				if self.0 {
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
		std::fs::remove_file(&from)
			.map(|_| None)
			.with_context(|| format!("could not delete {}", from.as_ref().display()))
	}
}

impl Trash {
	fn dir() -> Result<PathBuf> {
		let dir = dirs_next::data_local_dir().unwrap().join("organize").join(".trash");
		if !dir.exists() {
			std::fs::create_dir_all(&dir)?;
		}
		Ok(dir)
	}
}

impl Act for Trash {
	fn act<T, P>(&self, from: T, _to: Option<P>) -> Result<Option<PathBuf>>
	where
		T: AsRef<Path> + Into<PathBuf>,
		P: AsRef<Path> + Into<PathBuf>,
	{
		let to = Self::dir()?.join(from.as_ref().file_name().unwrap());
		std::fs::rename(&from, &to)
			.with_context(|| format!("could not move ({} -> {})", from.as_ref().display(), to.display()))
			.map(|_| None)
	}
}
