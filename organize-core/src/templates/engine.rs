use std::env::VarError;

use anyhow::Result;
use thiserror::Error;

use crate::{
	context::ExecutionContext,
	errors::Error,
	templates::template::{Piece, Template},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Templater;

#[derive(Error, Debug)]
pub enum TemplateError {
	#[error("variable '{variable}' (fields={fields:?}) cannot be resolved.")]
	UndefinedVariable {
		variable: String,
		fields: Vec<String>,
		#[source]
		source: VarError,
	},

	#[error("empty template")]
	EmptyTemplate,

	#[error(transparent)]
	Other(#[from] serde_json::Error),

	#[error("variable {variable} does not accept any fields, but received {fields:?}")]
	FieldsNotSupported { variable: String, fields: Vec<String> },

	#[error("invalid variable ({variable}): it requires {missing_piece} to be in scope")]
	InvalidContext { missing_piece: String, variable: String },

	#[error("variable '{variable}' does not support a '{field}' subfield")]
	InvalidField { variable: String, field: String },

	#[error("variable '{variable}' requires a field (one of: {fields})")]
	MissingField { variable: String, fields: String },

	#[error("unknown variable '{{{{ {0} }}}}'")]
	UnknownVariable(String),

	#[error("variable '{variable}' requires one of the following fields: {fields:?}")]
	RequiredField { variable: String, fields: Vec<String> },
}

impl Templater {
	pub fn new() -> Self {
		Templater::default()
	}

	pub async fn render(&self, template: &Template, ctx: &ExecutionContext<'_>) -> Result<String, Error> {
		let mut rendered = vec![];
		for piece in template.iter() {
			match piece {
				Piece::Literal(literal) => {
					rendered.push(literal.clone());
				}
				Piece::Variable(variable) => {
					let value = variable
						.compute(ctx)
						.await
						.inspect_err(|e| tracing::error!("{}", e.to_string()))?;
					rendered.push(value.as_str().expect("variables should return strings").to_string());
				}
			}
		}

		if rendered.is_empty() {
			return Err(Error::TemplateError(TemplateError::EmptyTemplate));
		}

		Ok(rendered.join(""))
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
