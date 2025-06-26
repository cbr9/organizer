use anyhow::Result;
use async_trait::async_trait;
use dyn_clone::DynClone;
use dyn_eq::DynEq;
use std::fmt::Debug;

use crate::{context::ExecutionContext, templates::engine::TemplateError};

// This enum represents the two possible outcomes of computing a variable.
#[derive(Debug)]
pub enum VariableOutput {
	// The variable resulted in a simple, final value.
	Value(serde_json::Value),
	// The variable resulted in a structured object that can be queried further.
	Lazy(Box<dyn Variable>),
}

dyn_clone::clone_trait_object!(Variable);
dyn_eq::eq_trait_object!(Variable);

#[async_trait]
#[typetag::serde(tag = "type")]
pub trait Variable: DynEq + DynClone + Sync + Send + Debug {
	fn name(&self) -> String;
	async fn compute(&self, parts: &[String], ctx: &ExecutionContext<'_>) -> Result<VariableOutput, TemplateError>;
}
