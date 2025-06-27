use anyhow::Result;
use async_trait::async_trait;
use dyn_clone::DynClone;
use dyn_eq::DynEq;
use std::fmt::Debug;

use crate::{context::ExecutionContext, errors::Error};

dyn_clone::clone_trait_object!(Variable);
dyn_eq::eq_trait_object!(Variable);

#[async_trait]
#[typetag::serde(tag = "type", content = "value")]
pub trait Variable: DynEq + DynClone + Sync + Send + Debug {
	fn name(&self) -> String;
	async fn compute(&self, ctx: &ExecutionContext<'_>) -> Result<serde_json::Value, Error>;
}
