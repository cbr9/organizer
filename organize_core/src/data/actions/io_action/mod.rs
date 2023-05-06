use std::{
	convert::TryFrom,
	path::{Path, PathBuf},
	result,
	str::FromStr,
};

use log::{error, info};
use serde::{Deserialize, Serialize};

use crate::{
	data::actions::{Act, ActionType, AsAction},
	path::{Expand, ResolveConflict},
	string::Placeholder,
	utils::UnwrapRef,
	// DB,
};
use anyhow::{Context, Result};

use regex::Regex;
use serde::de::Error;

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
			fn process<T: Into<PathBuf>>(&self, path: T) -> Option<PathBuf> {
				let path = path.into();
				let ty = self.ty();
				let to = self.0.prepare_path(&path, &ty);
				if to.is_none() {
					if self.0.if_exists == ConflictOption::Delete {
						if let Err(e) = std::fs::remove_file(&path).with_context(|| format!("could not delete {}", path.display())) {
							error!("{:?}", e);
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

				match self.act(&path, Some(to.unwrap_ref())) {
					Ok(new_path) => {
						info!("({}) {} -> {}", ty.to_string(), path.display(), to.unwrap().display());
						new_path
					}
					Err(e) => {
						error!("{:?}", e);
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
as_action!(Rename);
as_action!(Copy);
as_action!(Hardlink);
as_action!(Symlink);

impl Act for Move {
	fn act<T, P>(&self, from: T, to: Option<P>) -> Result<Option<PathBuf>>
	where
		T: AsRef<Path> + Into<PathBuf>,
		P: AsRef<Path> + Into<PathBuf>,
	{
		let to = to.unwrap().into();
		let from = from.as_ref();
		std::fs::rename(&from, &to)
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
		std::fs::copy(&from, &to)
			.with_context(|| "Failed to copy file")
			.map_or(Ok(None), |e| Ok(Some(from.into())))
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
	fn prepare_path<T>(&self, path: T, kind: &ActionType) -> Option<PathBuf>
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

		if to.exists() {
			to.resolve_naming_conflict(&self.if_exists)
		} else {
			Some(to)
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
