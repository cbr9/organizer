use crate::{context::ExecutionContext, templates::value::Value};
use anyhow::Result;
use async_trait::async_trait;
use dyn_clone::DynClone;
use dyn_eq::DynEq;
use std::fmt::Debug;

use super::function::TemplateFunction;

dyn_clone::clone_trait_object!(Accessor);
dyn_eq::eq_trait_object!(Accessor);

/// Represents a compiled and type-safe property path.
///
/// An Accessor is a function object that encapsulates the logic to retrieve a
/// specific value from a given execution context. This is the output of the
/// template compilation process.
#[async_trait]
pub trait Accessor: DynEq + DynClone + Sync + Send + Debug {
	async fn get(&self, ctx: &ExecutionContext) -> Result<Value>;
}

// New accessor for a literal value (like a string in a function call)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiteralAccessor {
	pub value: String,
}

#[async_trait]
impl Accessor for LiteralAccessor {
	async fn get(&self, _ctx: &ExecutionContext) -> Result<Value> {
		Ok(Value::String(self.value.clone()))
	}
}

// New accessor for a function call
#[derive(Debug, Clone)]
pub struct FunctionCallAccessor {
	pub function: Box<dyn TemplateFunction>,
	pub arg_accessors: Vec<Box<dyn Accessor>>,
}

// Manual impl of PartialEq because Box<dyn Trait> is not Eq
impl PartialEq for FunctionCallAccessor {
	fn eq(&self, other: &Self) -> bool {
		// This is a simplified comparison. A full comparison might not be possible.
		self.arg_accessors == other.arg_accessors
	}
}
impl Eq for FunctionCallAccessor {}

#[async_trait]
impl Accessor for FunctionCallAccessor {
	async fn get(&self, ctx: &ExecutionContext) -> Result<Value> {
		let mut args = Vec::new();
		for accessor in &self.arg_accessors {
			args.push(accessor.get(ctx).await?);
		}
		let value = self.function.call(ctx, args).await?;
		Ok(value)
	}
}
