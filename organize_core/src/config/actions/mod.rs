use std::fmt::Debug;
use std::path::PathBuf;

use anyhow::Result;
use dyn_clone::DynClone;
use dyn_eq::DynEq;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use script::ActionConfig;

use crate::resource::Resource;

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

dyn_clone::clone_trait_object!(Action);
dyn_eq::eq_trait_object!(Action);

#[typetag::serde(tag = "type")]
pub trait Action: DynEq + DynClone + Sync + Send + Debug {
	fn config(&self) -> ActionConfig;

	fn execute(&self, src: &Resource, dest: Option<PathBuf>, dry_run: bool) -> Result<Option<PathBuf>>;

	fn on_finish(&self, _resources: &[Resource], _dry_run: bool) -> Result<()> {
		Ok(())
	}

	// required only for some actions
	fn get_target_path(&self, _: &Resource) -> Result<Option<PathBuf>> {
		let config = self.config();
		if config.requires_dest {
			unimplemented!()
		}
		Ok(None)
	}

	fn run(&self, resources: Vec<Resource>, dry_run: bool) -> Vec<Resource> {
		let config = self.config();
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

	fn run_atomic(&self, mut resource: Resource, dry_run: bool) -> Option<Resource> {
		let config = self.config();
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
}
