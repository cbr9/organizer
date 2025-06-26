use std::{collections::HashMap, iter::FromIterator, pin::Pin};

use anyhow::Result;
use thiserror::Error;

use crate::{
	builtins::variables,
	config::Config,
	context::ExecutionContext,
	errors::Error,
	parser::ast::{Expression, Segment, VariablePath},
	templates::{
		template::Template,
		variable::{Variable, VariableOutput},
	},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Templater {
	pub global_variables: HashMap<String, Box<dyn Variable>>,
	pub rule_variables: HashMap<(usize, String), Box<dyn Variable>>,
}

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
	pub fn from_config(config: &Config) -> Self {
		let mut engine = Templater::default();
		for variable in config.variables.iter() {
			engine.global_variables.insert(variable.name(), variable.clone());
		}
		for (i, rule) in config.rules.iter().enumerate() {
			for variable in rule.variables.iter() {
				engine.rule_variables.insert((i, variable.name()), variable.clone());
			}
		}
		engine
	}

	async fn resolve_sub_variable(
		&self,
		variable: &Box<dyn Variable>,
		parts: &[String],
		ctx: &ExecutionContext<'_>,
	) -> Result<Pin<Box<VariableOutput>>, Error> {
		if parts.is_empty() {
			return Err(Error::TemplateError(TemplateError::EmptyTemplate));
		}
		let parts = &parts[1..];
		match variable.compute(parts, ctx).await? {
			VariableOutput::Value(value) => Ok(Box::pin(VariableOutput::Value(value))),
			VariableOutput::Lazy(variable) => Box::pin(self.resolve_sub_variable(&variable, parts, ctx)).await,
		}
	}

	pub async fn resolve_variable_path(&self, var_path: &VariablePath, ctx: &ExecutionContext<'_>) -> Result<serde_json::Value, Error> {
		let parts = &var_path.parts;
		let var_name = &parts[0];

		let Some(variable) = (match self.global_variables.get(var_name) {
			Some(var) => Some(var),
			None => self.rule_variables.get(&(ctx.scope.rule()?.index, var_name.clone())),
		}) else {
			return Err(Error::TemplateError(TemplateError::UnknownVariable(var_name.to_string())));
		};

		match &*self.resolve_sub_variable(variable, parts, ctx).await? {
			VariableOutput::Value(value) => Ok(value.clone()),
			VariableOutput::Lazy(_variable) => Err(Error::TemplateError(TemplateError::Other)),
		}
	}

	pub async fn render(&self, template: &Template, ctx: &ExecutionContext<'_>) -> Result<String, Error> {
		let mut rendered = String::new();
		for segment in template.ast.segments.iter() {
			match segment {
				Segment::Literal(literal) => {
					rendered += literal;
				}
				Segment::Expression(expression) => match expression {
					Expression::Variable(variable_path) => {
						let sub_rendered = self.resolve_variable_path(variable_path, ctx).await?;
						rendered += &serde_json::to_string(&sub_rendered).unwrap();
					}
				},
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
		let path: Box<dyn Variable> = Box::new(variables::path::Path);
		let root: Box<dyn Variable> = Box::new(variables::root::Root);
		let hash: Box<dyn Variable> = Box::new(variables::hash::Hash);
		let variables = HashMap::from_iter(vec![
			("path".to_string(), path.clone()),
			("root".to_string(), root.clone()),
			("file".to_string(), path),
			("hash".to_string(), hash),
		]);

		Self {
			global_variables: variables,
			rule_variables: HashMap::new(),
		}
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
