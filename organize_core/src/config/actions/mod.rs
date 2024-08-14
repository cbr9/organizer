use std::fmt::Debug;
use std::path::{Path, PathBuf};

use anyhow::Result;
use copy::Copy;
use delete::Delete;
use echo::Echo;
use email::Email;
use extract::Extract;
use hardlink::Hardlink;
use r#move::Move;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use script::{ActionConfig, Script};
use serde::Deserialize;
use symlink::Symlink;
use write::Write;

use crate::{config::actions::trash::Trash, resource::Resource};

use anyhow::Context;

pub mod common;
pub mod copy;
pub mod delete;
pub mod echo;
pub mod email;
pub mod extract;
pub mod hardlink;
pub mod r#move;
pub mod script;
pub mod symlink;
pub mod trash;
pub mod write;

pub trait ActionPipeline {
	fn run(&self, resources: Vec<Resource>, dry_run: bool) -> Vec<Resource>;
	fn run_atomic(&self, resource: Resource, dry_run: bool) -> Option<Resource>;
}

pub trait AsAction {
	const CONFIG: ActionConfig;

	fn execute<T: AsRef<Path>>(&self, src: &Resource, dest: Option<T>, dry_run: bool) -> Result<Option<PathBuf>>;

	fn on_finish(&self, _resources: &[Resource], _dry_run: bool) -> Result<()> {
		Ok(())
	}

	// required only for some actions
	fn get_target_path(&self, _: &Resource) -> Result<Option<PathBuf>> {
		let config = &Self::CONFIG;
		if config.requires_dest {
			unimplemented!()
		}
		Ok(None)
	}
}

impl<T: AsAction + Sync + Debug> ActionPipeline for T {
	#[tracing::instrument]
	#[allow(clippy::nonminimal_bool)]
	fn run_atomic(&self, mut resource: Resource, dry_run: bool) -> Option<Resource> {
		let config = Self::CONFIG;
		let mut value = Ok(None);
		if let Ok(dest) = self.get_target_path(&resource) {
			if (config.requires_dest && dest.is_some()) || (!config.requires_dest && dest.is_none()) {
				if !dry_run && dest.is_some() {
					let dest = dest.as_ref()?;
					let parent = dest.parent()?;
					if let Err(e) = std::fs::create_dir_all(parent).with_context(|| format!("Could not create {}", parent.display())) {
						tracing::error!("{:?}", e);
						return None;
					}
				}

				value = self.execute(&resource, dest, dry_run);
			}
		}

		if let Ok(value) = value {
			resource.set_path(value?);
			return Some(resource);
		}
		None
	}

	fn run(&self, resources: Vec<Resource>, dry_run: bool) -> Vec<Resource> {
		let config = Self::CONFIG;
		let resources: Vec<Resource> = if config.parallelize {
			resources
				.into_par_iter()
				.filter_map(|res| self.run_atomic(res, dry_run))
				.collect()
		} else {
			resources.into_iter().filter_map(|res| self.run_atomic(res, dry_run)).collect()
		};

		self.on_finish(&resources, dry_run).unwrap();
		resources
	}
}

impl ActionPipeline for Action {
	fn run(&self, resources: Vec<Resource>, dry_run: bool) -> Vec<Resource> {
		use Action::*;
		match self {
			Copy(copy) => copy.run(resources, dry_run),
			Move(r#move) => r#move.run(resources, dry_run),
			Hardlink(hardlink) => hardlink.run(resources, dry_run),
			Symlink(symlink) => symlink.run(resources, dry_run),
			Delete(delete) => delete.run(resources, dry_run),
			Echo(echo) => echo.run(resources, dry_run),
			Trash(trash) => trash.run(resources, dry_run),
			Script(script) => script.run(resources, dry_run),
			Extract(extract) => extract.run(resources, dry_run),
			Write(write) => write.run(resources, dry_run),
			Email(email) => email.run(resources, dry_run),
		}
	}

	fn run_atomic(&self, resource: Resource, dry_run: bool) -> Option<Resource> {
		use Action::*;
		match self {
			Copy(copy) => copy.run_atomic(resource, dry_run),
			Move(r#move) => r#move.run_atomic(resource, dry_run),
			Hardlink(hardlink) => hardlink.run_atomic(resource, dry_run),
			Symlink(symlink) => symlink.run_atomic(resource, dry_run),
			Delete(delete) => delete.run_atomic(resource, dry_run),
			Echo(echo) => echo.run_atomic(resource, dry_run),
			Trash(trash) => trash.run_atomic(resource, dry_run),
			Script(script) => script.run_atomic(resource, dry_run),
			Extract(extract) => extract.run_atomic(resource, dry_run),
			Write(write) => write.run_atomic(resource, dry_run),
			Email(email) => email.run_atomic(resource, dry_run),
		}
	}
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all(deserialize = "lowercase"))]
pub enum Action {
	Move(Move),
	Write(Write),
	Copy(Copy),
	Hardlink(Hardlink),
	Symlink(Symlink),
	Delete(Delete),
	Echo(Echo),
	Trash(Trash),
	Script(Script),
	Extract(Extract),
	Email(Email),
}
