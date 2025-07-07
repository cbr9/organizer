use anyhow::Result;
use async_trait::async_trait;
use organize_sdk::{
	context::ExecutionContext,
	error::Error,
	templates::{
		accessor::{Accessor, LiteralAccessor},
		compiler::TemplateCompiler,
		function::{FunctionInventory, TemplateFunctionBuilder},
		parser::Expression,
		value::Value,
	},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputBuilder;

static INPUT: InputBuilder = InputBuilder;
inventory::submit!(FunctionInventory { provider: &INPUT });

impl TemplateFunctionBuilder for InputBuilder {
	fn name(&self) -> &'static str {
		"input"
	}

	fn build(&self, compiler: &TemplateCompiler, mut args: Vec<Expression>) -> Result<Box<dyn Accessor>, Error> {
		// 1. Validate the number of arguments
		if args.len() > 1 {
			return Err(Error::Config(format!(
				"input() takes at most one argument (the prompt), but received {}",
				args.len()
			)));
		}

		// 2. Compile the argument expression into its own accessor.
		let prompt_accessor = if let Some(arg_expr) = args.pop() {
			compiler.build_accessor(arg_expr)?
		} else {
			// If no prompt is provided, create an accessor for the default prompt.
			Box::new(LiteralAccessor {
				value: "Enter a value:".to_string(),
			})
		};

		// 3. Return the specialized "Input" accessor, which holds its compiled argument.
		Ok(Box::new(InputAccessor { prompt_accessor }))
	}
}

/// This is the "parsed object" for an `input()` call.
/// It holds its specific arguments in a compiled, strongly-typed form.
#[derive(Debug, PartialEq, Eq, Clone)]
struct InputAccessor {
	prompt_accessor: Box<dyn Accessor>,
}

#[async_trait]
impl Accessor for InputAccessor {
	async fn get(&self, ctx: &ExecutionContext) -> Result<Value> {
		// First, execute the argument accessor to get the prompt string.
		let prompt_value = self.prompt_accessor.get(ctx).await?;
		let prompt = match prompt_value {
			Value::String(s) => s,
			other => return Err(Error::Config(format!("input() prompt must be a string, but received {:?}", other)).into()),
		};

		// Then, perform the function's logic.
		let input = ctx.services.reporter.ui.input(&prompt)?;
		Ok(Value::String(input))
	}
}
