use std::{fmt::Debug, path::PathBuf};

use anyhow::Result;
use dyn_clone::DynClone;
use dyn_eq::DynEq;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{
	config::context::Context,
	resource::Resource,
	templates::template::Template,
};

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

#[derive(Default)]
pub enum ExecutionModel {
	Linear,
	#[default]
	Parallel,
	Collection,
}

#[typetag::serde(tag = "type")]
pub trait Action: DynEq + DynClone + Sync + Send + Debug {
	fn execution_model(&self) -> ExecutionModel {
		ExecutionModel::default()
	}

	fn execute(&self, _res: &Resource, _ctx: &Context) -> Result<Option<PathBuf>> {
		unimplemented!("This action has not implemented `execute`.")
	}

	fn execute_collection(&self, _resources: Vec<&Resource>, _ctx: &Context) -> Result<Option<Vec<PathBuf>>> {
		unimplemented!("This action must be run in `Collection` mode and has not implemented `execute_collection`.")
	}

	fn templates(&self) -> Vec<&Template>;

	#[doc(hidden)]
	fn run(&self, mut resources: Vec<Resource>, ctx: &Context) -> Vec<Resource> {
		let filter_fn = |mut res| {
			let path = self.execute(&res, ctx).ok().flatten();
			if let Some(path) = path {
				res.set_path(path);
				Some(res)
			} else {
				None
			}
		};

		resources = match self.execution_model() {
			ExecutionModel::Linear => resources.into_iter().filter_map(filter_fn).collect(),
			ExecutionModel::Parallel => resources.into_par_iter().filter_map(filter_fn).collect(),
			ExecutionModel::Collection => todo!(),
		};

		resources
	}
}
