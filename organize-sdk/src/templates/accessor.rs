use crate::{context::ExecutionContext, templates::value::Value};
use anyhow::Result;
use async_trait::async_trait;
use dyn_clone::DynClone;
use dyn_eq::DynEq;
use std::fmt::Debug;

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
