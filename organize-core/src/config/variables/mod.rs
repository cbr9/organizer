use async_trait::async_trait;
use dyn_clone::DynClone;
use dyn_eq::DynEq;
use std::fmt::Debug;

use crate::{config::context::ExecutionContext, templates::template::Template};

pub mod builtins;
pub mod custom;

dyn_clone::clone_trait_object!(Variable);
dyn_eq::eq_trait_object!(Variable);

#[async_trait]
#[typetag::serde(tag = "type")]
pub trait Variable: DynEq + DynClone + Sync + Send + Debug {
	fn name(&self) -> Option<&str> {
		None
	}
	fn templates(&self) -> Vec<&Template> {
		vec![]
	}

	/// Lazily computes a single value when requested by a template.
	/// This should return an error if the computation fails.
	async fn compute(&self, ctx: &ExecutionContext<'_>) -> anyhow::Result<tera::Value>;
}
