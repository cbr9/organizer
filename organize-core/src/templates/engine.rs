use std::{collections::HashMap, iter::FromIterator, pin::Pin};

use anyhow::Result;
use thiserror::Error;

use crate::{
	builtins::variables,
	config::Config,
	context::ExecutionContext,
	errors::Error,
	parser::ast::{Expression, Segment},
	templates::{
		template::{BuiltSegment, Template},
		variable::{Variable, VariableOutput},
	},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Templater;

#[derive(Error, Debug)]
pub enum TemplateError {
	#[error("This name of this rule's variable is not unique with respect to other builtin variables and other globally defined variables")]
	NonUniqueNames { name: String, rule_index: usize },
	#[error("Empty template")]
	EmptyTemplate,
	#[error("Unexpected error")]
	Other,
	#[error("Unknown variable {0}")]
	UnknownVariable(String),
	#[error("Invalid variable ({variable}): it requires {missing_piece} to be in scope")]
	InvalidContext { missing_piece: String, variable: String },
}

impl Templater {
	pub fn new() -> Self {
		Templater::default()
	}

	async fn resolve_sub_variable(
		&self,
		variable: &Box<dyn Variable>,
		parts: &mut Vec<String>,
		ctx: &ExecutionContext<'_>,
	) -> Result<Pin<Box<VariableOutput>>, Error> {
		if parts.len() > 0 {
			parts.remove(0);
		}
		match variable.compute(parts, ctx).await? {
			VariableOutput::Value(value) => Ok(Box::pin(VariableOutput::Value(value))),
			VariableOutput::Lazy(variable) => Box::pin(self.resolve_sub_variable(&variable, parts, ctx)).await,
		}
	}

	pub async fn resolve_variable_path(
		&self,
		variable: &Box<dyn Variable>,
		parts: &Vec<String>,
		ctx: &ExecutionContext<'_>,
	) -> Result<serde_json::Value, Error> {
		let mut parts = parts.clone();
		match &*self.resolve_sub_variable(variable, &mut parts, ctx).await? {
			VariableOutput::Value(value) => Ok(value.clone()),
			VariableOutput::Lazy(_variable) => unreachable!("Template should evaluate fully"),
		}
	}

	pub async fn render(&self, template: &Template, ctx: &ExecutionContext<'_>) -> Result<String, Error> {
		let mut rendered = String::new();
		for segment in template.variables.iter() {
			match segment {
				BuiltSegment::Literal(literal) => {
					rendered += literal;
				}
				BuiltSegment::Expression(variable, parts) => {
					let value = self.resolve_variable_path(variable, parts, ctx).await?;
					rendered += &serde_json::to_string_pretty(&value)?;
				}
			}
		}

		if rendered.is_empty() {
			return Err(Error::TemplateError(TemplateError::EmptyTemplate));
		}

		Ok(rendered.replace("\"", ""))
	}
}

impl Default for Templater {
	/// Creates a new Templater with all built-in filters and lazy variables.
	fn default() -> Self {
		// Initialize the templater with the complete set of built-in lazy variables
		Self {}
	}
}
// #[cfg(test)]
// mod tests {
// 	use std::convert::{TryFrom, TryInto};

// 	use super::*;
// 	use crate::{config::{context::RunServices, variables::simple::SimpleVariable}, resource::Resource};

// 	#[test]
// 	fn render_template_not_present_in_engine() {
// 		let engine = Templater::default();
// 		let template = Template::try_from("Hello, {{ name }}!").unwrap();
// 		let context = Context::new(ctx)
// 		let mut context = engine.context().build(&engine);
// 		context.insert("name", "Andrés");
// 		let rendered = engine.render(&template, &context);
// 		assert!(rendered.is_err());
// 	}

// 	#[test]
// 	fn render_template_present_in_engine() {
// 		let mut engine = Templater::default();
// 		let template = Template::try_from("This is a stored template.").unwrap();
// 		engine.add_template(&template).unwrap();
// 		let context = engine.context().build(&engine);
// 		let rendered = engine.render(&template, &context).unwrap();
// 		assert_eq!(rendered, Some("This is a stored template.".to_string()));
// 	}

// 	#[test]
// 	fn render_with_simple_variable() {
// 		let var = SimpleVariable {
// 			name: "location".into(),
// 			value: "world".try_into().unwrap(),
// 		};
// 		let mut engine = Templater::new(&vec![Box::new(var)]);
// 		let template = Template::try_from("Hello, {{ location }}!").unwrap();
// 		engine.add_template(&template).unwrap();
// 		let context = engine.context().build(&engine);
// 		let rendered = engine.render(&template, &context).unwrap();
// 		assert_eq!(rendered, Some("Hello, world!".to_string()));
// 	}

// 	#[test]
// 	fn render_with_path_context() {
// 		let mut engine = Templater::default();
// 		let resource = Resource::new_tmp("test.txt");
// 		let template = Template::try_from("The path is {{ path | stem }}").unwrap();
// 		engine.add_template(&template).unwrap();
// 		let context = engine.context().path(resource.path().to_path_buf()).build(&engine);
// 		let rendered = engine.render(&template, &context).unwrap();
// 		assert_eq!(rendered, Some("The path is test".to_string()));
// 	}

// 	#[test]
// 	fn render_invalid_template_returns_none() {
// 		let engine = Templater::default();
// 		// Invalid syntax: `{%` instead of `{{`
// 		let template = Template::try_from("Hello, {% name }}!").unwrap();
// 		let context = engine.context().build(&engine);
// 		let rendered = engine.render(&template, &context);
// 		assert!(rendered.is_err());
// 	}
// }
