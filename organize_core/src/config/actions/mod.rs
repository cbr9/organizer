use std::path::{Path, PathBuf};

use copy::Copy;
use delete::Delete;
use echo::Echo;
use extract::Extract;
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
pub mod extract;
pub(crate) mod hardlink;
pub(crate) mod r#move;
pub(crate) mod script;
pub(crate) mod symlink;
pub(crate) mod trash;

pub trait ActionRunner {
	fn run<T: AsRef<Path> + Into<PathBuf> + Clone>(&self, src: T, simulated: bool) -> Result<Option<PathBuf>>;
}

impl<T: ActionPipeline> ActionRunner for T {
	#[allow(clippy::nonminimal_bool)]
	fn run<P: AsRef<Path> + Into<PathBuf> + Clone>(&self, src: P, simulated: bool) -> Result<Option<PathBuf>> {
		let dest = self.get_target_path(src.clone());
		if let Ok(dest) = dest {
			if (Self::REQUIRES_DEST && dest.is_some()) || !Self::REQUIRES_DEST {
				if Self::REQUIRES_DEST && dest.is_some() && src.as_ref() == dest.as_ref().unwrap() {
					return Ok(dest);
				}
				let confirmation = self.confirm(src.clone(), dest.clone())?;
				let src = src.into();

				if confirmation {
					return match self.execute(src.clone(), dest.clone(), simulated) {
						Ok(new_path) => {
							log::info!("{}", self.log_success_msg(src, new_path.clone(), simulated)?);
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
	fn run<T: AsRef<Path> + Into<PathBuf> + Clone>(&self, src: T, simulated: bool) -> Result<Option<PathBuf>> {
		use Action::*;
		match self {
			Copy(copy) => copy.run(src, simulated),
			Move(r#move) => r#move.run(src, simulated),
			Hardlink(hardlink) => hardlink.run(src, simulated),
			Symlink(symlink) => symlink.run(src, simulated),
			Delete(delete) => delete.run(src, simulated),
			Echo(echo) => echo.run(src, simulated),
			Trash(trash) => trash.run(src, simulated),
			Script(script) => script.run(src, simulated),
			Extract(extract) => extract.run(src, simulated),
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
		simulated: bool,
	) -> Result<Option<PathBuf>>;

	fn log_success_msg<T: AsRef<Path> + Into<PathBuf> + Clone, P: AsRef<Path> + Into<PathBuf> + Clone>(
		&self,
		src: T,
		dest: Option<P>,
		simulated: bool,
	) -> Result<String> {
		use ActionType::*;
		let hint = if !simulated {
			Self::TYPE.to_string().to_uppercase()
		} else {
			format!("SIMULATED {}", Self::TYPE.to_string().to_uppercase())
		};
		match Self::TYPE {
			Copy | Move | Hardlink | Symlink | Extract => Ok(format!(
				"({}) {} -> {}",
				hint,
				src.as_ref().display(),
				dest.expect("dest should not be none").as_ref().display()
			)),
			Delete | Trash => Ok(format!("({}) {}", hint, src.as_ref().display())),
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
	Extract(Extract),
}

#[derive(Eq, PartialEq, Display, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum ActionType {
	Copy,
	Extract,
	Delete,
	Echo,
	Move,
	Hardlink,
	Symlink,
	Script,
	Trash,
}
