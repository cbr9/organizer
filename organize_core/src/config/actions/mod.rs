use std::path::{Path, PathBuf};

use copy::Copy;
use delete::Delete;
use echo::Echo;
use hardlink::Hardlink;
use r#move::Move;
use script::Script;
use serde::Deserialize;
use strum_macros::{Display, EnumString};
use symlink::Symlink;

use crate::config::actions::trash::Trash;

use anyhow::Result;

pub(crate) mod common;
pub(crate) mod copy;
pub(crate) mod delete;
pub(crate) mod echo;
pub(crate) mod hardlink;
pub(crate) mod r#move;
pub(crate) mod script;
pub(crate) mod symlink;
pub(crate) mod trash;

pub trait ActionRunner {
	fn run<T: AsRef<Path> + Into<PathBuf> + Clone>(&self, src: T) -> Result<Option<PathBuf>>;
}

impl<T: ActionPipeline> ActionRunner for T {
	fn run<P: AsRef<Path> + Into<PathBuf> + Clone>(&self, src: P) -> Result<Option<PathBuf>> {
		let dest = self.get_target_path(src.clone());
		if let Ok(dest) = dest {
			if (Self::REQUIRES_DEST && dest.is_some()) || !Self::REQUIRES_DEST {
				let confirmation = self.confirm(src.clone(), dest.clone())?;
				let src = src.into();

				if confirmation {
					return match self.execute(src.clone(), dest.clone()) {
						Ok(new_path) => {
							log::info!("{}", self.log_success_msg(src, new_path.clone())?);
							Ok(new_path)
						}
						Err(e) => {
							log::error!("{:?}", e);
							Err(e)
						}
					};
				}
			}
		}
		Ok(None)
	}
}

impl ActionRunner for Action {
	fn run<T: AsRef<Path> + Into<PathBuf> + Clone>(&self, src: T) -> Result<Option<PathBuf>> {
		use Action::*;
		match self {
			Copy(copy) => copy.run(src),
			Move(r#move) => r#move.run(src),
			Hardlink(hardlink) => hardlink.run(src),
			Symlink(symlink) => symlink.run(src),
			Delete(delete) => delete.run(src),
			Echo(echo) => echo.run(src),
			Trash(trash) => trash.run(src),
			Script(script) => script.run(src),
		}
	}
}

pub trait ActionPipeline {
	const TYPE: ActionType;
	const REQUIRES_DEST: bool;
	fn execute<T: AsRef<Path> + Into<PathBuf> + Clone, P: AsRef<Path> + Into<PathBuf> + Clone>(
		&self,
		src: T,
		dest: Option<P>,
	) -> Result<Option<PathBuf>>;

	fn log_success_msg<T: AsRef<Path> + Into<PathBuf> + Clone, P: AsRef<Path> + Into<PathBuf> + Clone>(
		&self,
		src: T,
		dest: Option<P>,
	) -> Result<String> {
		use ActionType::*;
		match Self::TYPE {
			Copy | Move | Hardlink | Symlink => Ok(format!(
				"({}) {} -> {}",
				Self::TYPE,
				src.as_ref().display(),
				dest.unwrap().as_ref().display()
			)),
			Delete | Trash => Ok(format!("({}) {}", Self::TYPE, src.as_ref().display())),
			_ => unimplemented!(),
		}
	}

	// required only for some actions
	fn confirm<T: AsRef<Path> + Into<PathBuf> + Clone, P: AsRef<Path> + Into<PathBuf> + Clone>(&self, _: T, _: Option<P>) -> Result<bool> {
		Ok(true)
	}

	// required only for some actions
	fn get_target_path<T: AsRef<Path> + Into<PathBuf> + Clone>(&self, _: T) -> Result<Option<PathBuf>> {
		if Self::REQUIRES_DEST {
			unimplemented!()
		}
		Ok(None)
	}
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all(deserialize = "lowercase"))]
pub enum Action {
	Move(Move),
	Copy(Copy),
	Hardlink(Hardlink),
	Symlink(Symlink),
	Delete(Delete),
	Echo(Echo),
	Trash(Trash),
	Script(Script),
}

#[derive(Eq, PartialEq, Display, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum ActionType {
	Copy,
	Delete,
	Echo,
	Move,
	Hardlink,
	Symlink,
	Script,
	Trash,
}
