use std::{
	convert::TryFrom,
	path::{Path, PathBuf},
	result,
	str::FromStr,
};

use derive_more::Deref;
use serde::{Deserialize, Serialize};

use crate::{
	config::actions::{Act, ActionType, AsAction},
	path::{Expand, ResolveConflict},
	string::ExpandPlaceholder,
	utils::UnwrapRef,
	// DB,
};
use anyhow::{bail, Context, Result};

use regex::Regex;
use serde::de::Error;

#[derive(Deserialize, Debug, Clone, Eq, PartialEq, Default)]
pub struct Inner {
	pub to: PathBuf,
	#[serde(default)]
	pub if_exists: ConflictOption,
	#[serde(default)]
	pub allow_cycles: bool,
}

#[derive(Deserialize, Deref, Debug, Clone, PartialEq, Eq)]
pub struct Move(Inner);

#[derive(Deserialize, Deref, Debug, Clone, PartialEq, Eq)]
pub struct Copy(Inner);

#[derive(Deserialize, Deref, Debug, Clone, PartialEq, Eq)]
pub struct Hardlink(Inner);

#[derive(Deserialize, Deref, Debug, Clone, PartialEq, Eq)]
pub struct Symlink(Inner);

macro_rules! as_action {
	($id:ty) => {
		impl AsAction for $id {
			fn process<T: Into<PathBuf>>(&self, path: T) -> Option<PathBuf> {
				let path = path.into();
				let to = self.0.prepare_path(&path);
				if to.is_none() {
					if self.0.if_exists == ConflictOption::Delete {
						if let Err(e) = std::fs::remove_file(&path).with_context(|| format!("could not delete {}", path.display())) {
							log::error!("{:?}", e);
						}
					}
					return None;
				}

				match to.unwrap_ref().parent() {
					Some(parent) => {
						if !parent.exists() {
							if let Err(e) = std::fs::create_dir_all(parent)
								.with_context(|| format!("could not create parent directory for {}", to.unwrap_ref().display()))
							{
								log::error!("{:?}", e);
								return None;
							}
						}
					}
					None => {
						log::error!("{} has an invalid parent", to.unwrap().display());
						return None;
					}
				}

				match self.act(&path, Some(to.unwrap_ref())) {
					Ok(new_path) => {
						log::info!("({}) {} -> {}", self.ty().to_string(), path.display(), to.unwrap().display());
						new_path
					}
					Err(e) => {
						log::error!("{:?}", e);
						None
					}
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
as_action!(Copy);
as_action!(Hardlink);
as_action!(Symlink);

impl Act for Move {
	fn act<T, P>(&self, from: T, to: Option<P>) -> Result<Option<PathBuf>>
	where
		T: AsRef<Path> + Into<PathBuf>,
		P: AsRef<Path> + Into<PathBuf>,
	{
		let to = Into::<PathBuf>::into(to.unwrap());
		let from = from.as_ref();
		if to.parent().unwrap() == from.parent().unwrap() && !self.allow_cycles {
			bail!(
				"Origin {} and target {} paths are inside the same folder, but cycles are not allowed",
				from.display(),
				&to.display()
			)
		}
		std::fs::rename(from, &to)
			.with_context(|| "Failed to move file")
			.map_or(Ok(None), |_| Ok(Some(to)))
	}
}

impl Act for Copy {
	fn act<T, P>(&self, from: T, to: Option<P>) -> Result<Option<PathBuf>>
	where
		T: AsRef<Path> + Into<PathBuf>,
		P: AsRef<Path> + Into<PathBuf>,
	{
		let to = to.unwrap().into();
		let from = from.as_ref();
		if !self.allow_cycles {
			if to.parent().unwrap() == from.parent().unwrap() {
				bail!(
					"Origin {} and target {} paths are inside the same folder, but cycles are not allowed",
					from.display(),
					&to.display()
				)
			}
		}
		std::fs::copy(from, to)
			.with_context(|| "Failed to copy file")
			.map_or(Ok(None), |_| Ok(Some(from.into())))
	}
}

impl Act for Hardlink {
	fn act<T, P>(&self, from: T, to: Option<P>) -> Result<Option<PathBuf>>
	where
		T: AsRef<Path> + Into<PathBuf>,
		P: AsRef<Path> + Into<PathBuf>,
	{
		let to = to.unwrap().into();
		let from = from.as_ref();
		if !self.allow_cycles {
			if to.parent().unwrap() == from.parent().unwrap() {
				bail!(
					"Origin {} and target {} paths are inside the same folder, but cycles are not allowed",
					from.display(),
					to.display()
				)
			}
		}
		std::fs::hard_link(&from, &to)
			.with_context(|| format!("could not create hardlink ({} -> {})", from.display(), to.display()))
			.map(|_| Some(from.into()))
	}
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
impl Act for Symlink {
	fn act<T, P>(&self, from: T, to: Option<P>) -> Result<Option<PathBuf>>
	where
		T: AsRef<Path> + Into<PathBuf>,
		P: AsRef<Path> + Into<PathBuf>,
	{
		let to = to.unwrap().into();
		let from = from.as_ref();
		if !self.allow_cycles {
			if to.parent().unwrap() == from.parent().unwrap() {
				bail!(
					"Origin {} and target {} paths are inside the same folder, but cycles are not allowed",
					from.display(),
					to.display()
				)
			}
		}
		std::os::unix::fs::symlink(from, &to)
			.with_context(|| format!("could not create symlink ({} -> {})", from.display(), to.display()))
			.map(|_| Some(from.into()))
	}
}

impl Inner {
	fn prepare_path<T>(&self, path: T) -> Option<PathBuf>
	where
		T: AsRef<Path>,
	{
		let path = path.as_ref();
		let mut to = match self.to.to_string_lossy().expand_placeholders(path) {
			Ok(str) => PathBuf::from(str),
			Err(e) => {
				log::error!("{:?}", e);
				return None;
			}
		};

		if to.extension().is_none() || to.is_dir() {
			to.push(path.file_name()?)
		}

		match to.exists() {
			true => to.resolve_naming_conflict(&self.if_exists),
			false => Some(to),
		}
	}
}

impl TryFrom<PathBuf> for Inner {
	type Error = anyhow::Error;

	fn try_from(value: PathBuf) -> result::Result<Self, Self::Error> {
		let action = Self {
			to: value.expand_user()?.expand_vars()?,
			if_exists: Default::default(),
			allow_cycles: false,
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
#[derive(Eq, PartialEq, Default, Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum ConflictOption {
	Overwrite,
	Skip,
	#[default]
	Rename,
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
			_ => panic!("Unknown option"),
		};
		Ok(variant)
	}
}
