use anyhow::Result;
use async_trait::async_trait;
use dyn_clone::DynClone;
use dyn_eq::DynEq;
use serde_json::Value;
use std::fmt::Debug;

use crate::template::Template;

use std::sync::Arc;

// This enum represents the two possible outcomes of computing a variable.
pub enum VariableOutput {
	// The variable resulted in a simple, final value.
	Value(serde_json::Value),
	// The variable resulted in a structured object that can be queried further.
	Lazy(Arc<dyn LazyObject>),
}

// The contract for any object that has fields that can be accessed lazily.
#[async_trait]
pub trait LazyObject: Send + Sync {
	async fn get(&self, name: &str) -> Result<Value>;
}

dyn_clone::clone_trait_object!(Variable);
dyn_eq::eq_trait_object!(Variable);

#[async_trait]
#[typetag::serde(tag = "type")]
pub trait Variable: DynEq + DynClone + Sync + Send + Debug {
	fn name(&self) -> String;
	fn templates(&self) -> Vec<&Template>;
	async fn compute(&self) -> Result<VariableOutput>;
}
