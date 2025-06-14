use std::fmt::Debug;
use std::path::PathBuf;

use anyhow::Result;
use dyn_clone::DynClone;
use dyn_eq::DynEq;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{
	resource::Resource,
	templates::{template::Template, TemplateEngine},
};

use super::variables::Variable;

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

#[derive(Debug)]
pub struct ActionConfig {
	pub parallelize: bool,
}

#[typetag::serde(tag = "type")]
pub trait Action: DynEq + DynClone + Sync + Send + Debug {
	fn config(&self) -> ActionConfig;

	fn execute(&self, src: &Resource, template_engine: &TemplateEngine, variables: &[Box<dyn Variable>], dry_run: bool) -> Result<Option<PathBuf>>;

	fn on_finish(&self, _resources: &[Resource], _dry_run: bool) -> Result<()> {
		Ok(())
	}

	fn templates(&self) -> Vec<Template>;

	#[doc(hidden)]
	fn run(&self, resources: Vec<Resource>, template_engine: &TemplateEngine, variables: &[Box<dyn Variable>], dry_run: bool) -> Vec<Resource> {
		let config = self.config();

		let filter_fn = |mut res| {
			let path = self
				.execute(&res, template_engine, variables, dry_run)
				.inspect_err(|e| tracing::error!("{}", e))
				.ok()
				.flatten();
			if let Some(path) = path {
				res.set_path(path);
				Some(res)
			} else {
				None
			}
		};

		let resources: Vec<Resource> = if config.parallelize {
			resources.into_par_iter().filter_map(filter_fn).collect()
		} else {
			resources.into_iter().filter_map(filter_fn).collect()
		};

		self.on_finish(&resources, dry_run).unwrap();
		resources
	}
}
