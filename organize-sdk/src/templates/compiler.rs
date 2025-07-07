use crate::{
	error::Error,
	templates::{
		accessor::Accessor,
		registry::VariableRegistry,
		template::{Template, TemplatePart},
	},
};
use anyhow::Result;

use super::{
	accessor::LiteralAccessor,
	parser::{Expression, Segment, AST},
	registry::FunctionRegistry,
};

/// The central compiler for the template system.
/// It uses a SchemaRegistry to validate variables and a full parser to build the template.
#[derive(Debug, Clone)]
pub struct TemplateCompiler {
	variables: VariableRegistry,
	functions: FunctionRegistry,
}

impl Default for TemplateCompiler {
	fn default() -> Self {
		Self::new()
	}
}

impl TemplateCompiler {
	/// Creates a new compiler with a default schema registry that discovers
	/// all registered static variable providers.
	pub fn new() -> Self {
		Self {
			variables: VariableRegistry::new(),
			functions: FunctionRegistry::new(),
		}
	}

	/// Compiles a raw string into an executable Template object using your parser.
	pub fn compile_template(&self, raw_template: &str) -> Result<Template, Error> {
		// Stage 1: Parse the raw string into an Abstract Syntax Tree (AST)
		// using your provided `AST::parse` method.
		let ast = AST::parse(raw_template)?;
		let mut parts = Vec::new();

		// Stage 2: Walk the AST to build the final, executable Template object.
		for segment in ast.segments {
			match segment {
				Segment::Literal(text) => {
					parts.push(TemplatePart::Static(text));
				}
				Segment::Expression(expr) => {
					let accessor = self.build_accessor(expr)?;
					parts.push(TemplatePart::Dynamic(accessor));
				}
			}
		}

		Ok(Template {
			parts,
			text: raw_template.to_string(),
		})
	}

	/// Builds a type-safe accessor from a parsed expression AST node.
	/// This is the bridge between your parser and the execution engine.
	pub fn build_accessor(&self, expr: Expression) -> Result<Box<dyn Accessor>, Error> {
		match expr {
			Expression::Variable(parts) => {
				let parts_str: Vec<&str> = parts.iter().map(AsRef::as_ref).collect();
				self.variables.parse_property_chain(&parts_str)
			}
			Expression::Literal(value) => Ok(Box::new(LiteralAccessor { value })),
			Expression::FunctionCall { name, args } => {
				let builder = self
					.functions
					.get(&name)
					.ok_or_else(|| Error::TemplateError(super::engine::TemplateError::UnknownVariable(format!("Unknown function '{name}'"))))?;

				// Delegate the argument parsing and compilation to the function's own builder
				builder.build(self, args)
			}
		}
	}
}
