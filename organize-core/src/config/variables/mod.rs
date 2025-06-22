use dyn_clone::DynClone;
use dyn_eq::DynEq;
use std::fmt::Debug;

use crate::{config::context::ExecutionContext, templates::template::Template};

// pub mod regex;
pub mod simple;

dyn_clone::clone_trait_object!(Variable);
dyn_eq::eq_trait_object!(Variable);

#[typetag::serde(tag = "type")]
pub trait Variable: DynEq + DynClone + Sync + Send + Debug {
	fn name(&self) -> &str;
	fn templates(&self) -> Vec<&Template>;

	/// Lazily computes a single value when requested by a template.
	/// This should return an error if the computation fails.
	fn compute(&self, ctx: &ExecutionContext<'_>) -> anyhow::Result<tera::Value>;
}
