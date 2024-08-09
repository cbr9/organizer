use std::path::{Path, PathBuf};

use copy::Copy;
use delete::Delete;
use echo::Echo;
use extract::Extract;
use hardlink::Hardlink;
use r#move::Move;
use script::{ActionConfig, Script};
use serde::Deserialize;
use symlink::Symlink;

use crate::{config::actions::trash::Trash, resource::Resource};

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

pub trait ActionPipeline {
	fn run(&self, src: &Resource, dry_run: bool) -> Result<Option<PathBuf>>;
}

impl<T: AsAction> ActionPipeline for T {
	#[allow(clippy::nonminimal_bool)]
	fn run(&self, src: &Resource, dry_run: bool) -> Result<Option<PathBuf>> {
		let dest = self.get_target_path(src)?;
		let config = Self::CONFIG;
		if (config.requires_dest && dest.is_some()) || (!config.requires_dest && dest.is_none()) {
			if config.requires_dest && dest.is_some() && &src.path == dest.as_ref().unwrap() {
				return Ok(dest);
			}

			if !dry_run && dest.is_some() {
				let dest = dest.clone().unwrap();
				std::fs::create_dir_all(dest.parent().unwrap())?;
			}

			return self.execute(src, dest.clone(), dry_run);
		}
		Ok(None)
	}
}

impl ActionPipeline for Action {
	fn run(&self, src: &Resource, dry_run: bool) -> Result<Option<PathBuf>> {
		use Action::*;
		match self {
			Copy(copy) => copy.run(src, dry_run),
			Move(r#move) => r#move.run(src, dry_run),
			Hardlink(hardlink) => hardlink.run(src, dry_run),
			Symlink(symlink) => symlink.run(src, dry_run),
			Delete(delete) => delete.run(src, dry_run),
			Echo(echo) => echo.run(src, dry_run),
			Trash(trash) => trash.run(src, dry_run),
			Script(script) => script.run(src, dry_run),
			Extract(extract) => extract.run(src, dry_run),
		}
	}
}

pub trait AsAction {
	const CONFIG: ActionConfig;

	fn execute<T: AsRef<Path>>(&self, src: &Resource, dest: Option<T>, dry_run: bool) -> Result<Option<PathBuf>>;

	// required only for some actions
	fn get_target_path(&self, _: &Resource) -> Result<Option<PathBuf>> {
		let config = &Self::CONFIG;
		if config.requires_dest {
			unimplemented!()
		}
		Ok(None)
	}
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
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
