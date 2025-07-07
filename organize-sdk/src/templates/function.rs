use crate::{
	context::ExecutionContext,
	error::Error,
	templates::{accessor::Accessor, compiler::TemplateCompiler, parser::Expression},
};
use async_trait::async_trait;
use dyn_clone::DynClone;
use dyn_eq::DynEq;
use inventory;
use std::fmt::Debug;

use super::value::Value;

dyn_clone::clone_trait_object!(TemplateFunction);
dyn_eq::eq_trait_object!(TemplateFunction);

#[async_trait]
pub trait TemplateFunction: Send + Sync + Debug + DynClone + DynEq {
	fn name(&self) -> &'static str;
	async fn call(&self, ctx: &ExecutionContext, args: Vec<Value>) -> Result<Value, Error>;
}

dyn_clone::clone_trait_object!(TemplateFunctionBuilder);
dyn_eq::eq_trait_object!(TemplateFunctionBuilder);

/// A trait for building a function call accessor from the template AST.
/// Each function (like `input` or a future `now`) will have its own builder.
pub trait TemplateFunctionBuilder: Send + Sync + Debug + DynClone + DynEq {
	/// The public name of the function in the template language.
	fn name(&self) -> &'static str;

	/// Takes the template compiler and the function's arguments from the AST,
	/// and returns a compiled, executable Accessor for this specific function call.
	fn build(&self, compiler: &TemplateCompiler, args: Vec<Expression>) -> Result<Box<dyn Accessor>, Error>;
}

pub struct FunctionInventory {
	pub provider: &'static (dyn TemplateFunctionBuilder + Sync),
}

inventory::collect!(FunctionInventory);
