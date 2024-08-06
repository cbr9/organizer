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

use crate::{
	config::{actions::trash::Trash, SIMULATION},
	resource::Resource,
};

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
	fn run(&self, src: &Resource) -> Result<Option<PathBuf>>;
}

impl<T: ActionPipeline> ActionRunner for T {
	#[allow(clippy::nonminimal_bool)]
	fn run(&self, src: &Resource) -> Result<Option<PathBuf>> {
		let dest = self.get_target_path(src);
		if let Ok(dest) = dest {
			if (Self::REQUIRES_DEST && dest.is_some()) || !Self::REQUIRES_DEST {
				if Self::REQUIRES_DEST && dest.is_some() && src.path().as_ref() == dest.as_ref().unwrap() {
					return Ok(dest);
				}

				if !*SIMULATION && dest.is_some() {
					let dest = dest.clone().unwrap();
					std::fs::create_dir_all(dest.parent().unwrap())?;
				}
				return match self.execute(src, dest.clone()) {
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
		Ok(None)
	}
}

impl ActionRunner for Action {
	fn run(&self, src: &Resource) -> Result<Option<PathBuf>> {
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
			Extract(extract) => extract.run(src),
		}
	}
}

pub trait ActionPipeline {
	const TYPE: ActionType;
	const REQUIRES_DEST: bool;

	fn execute<T: AsRef<Path>>(&self, src: &Resource, dest: Option<T>) -> Result<Option<PathBuf>>;

	fn log_success_msg<T: AsRef<Path>>(&self, src: &Resource, dest: Option<T>) -> Result<String> {
		use ActionType::*;
		let hint = if !*SIMULATION {
			Self::TYPE.to_string().to_uppercase()
		} else {
			format!("SIMULATED {}", Self::TYPE.to_string().to_uppercase())
		};
		match Self::TYPE {
			Copy | Move | Hardlink | Symlink | Extract => Ok(format!(
				"({}) {} -> {}",
				hint,
				src.path().display(),
				dest.expect("dest should not be none").as_ref().display()
			)),
			Delete | Trash => Ok(format!("({}) {}", hint, src.path().display())),
			_ => unimplemented!(),
		}
	}

	// required only for some actions
	fn get_target_path(&self, _: &Resource) -> Result<Option<PathBuf>> {
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
