use std::collections::HashSet;

use serde::{Deserialize, Deserializer, Serialize};
use tera::ast::{Expr, ExprVal, FunctionCall, Node};
use uuid::Uuid;

#[derive(Serialize, Default, Debug, Clone)]
pub struct Template {
	pub id: String,
	pub input: String,
	pub dependencies: HashSet<String>,
}

impl Template {
	fn get_dependencies(template: &tera::Template) -> HashSet<String> {
		let mut deps = HashSet::new();
		walk_nodes(&template.ast, &mut deps);
		deps.into_iter().collect()
	}
}

/// Recursively walks through a sequence of top-level AST nodes.
/// It looks for nodes that can contain expressions, like `{% if %}` or `{{ }}`.
fn walk_nodes(nodes: &[Node], deps: &mut HashSet<String>) {
	for node in nodes {
		match node {
			Node::VariableBlock(_, expr) => walk_expr(expr, deps),
			Node::If(if_node, _) => {
				for (_, condition, body) in &if_node.conditions {
					walk_expr(condition, deps);
					walk_nodes(body, deps);
				}
				if let Some((_, else_body)) = &if_node.otherwise {
					walk_nodes(else_body, deps);
				}
			}
			Node::Forloop(_, for_node, _) => {
				walk_expr(&for_node.container, deps);
				walk_nodes(&for_node.body, deps);
				if let Some(empty_body) = &for_node.empty_body {
					walk_nodes(empty_body, deps);
				}
			}
			Node::Set(_, set_node) => walk_expr(&set_node.value, deps),
			Node::FilterSection(_, section, _) => {
				walk_function_call(&section.filter, deps);
				walk_nodes(&section.body, deps);
			}
			Node::Block(_, block, _) => walk_nodes(&block.body, deps),
			_ => (), // Other nodes like Text, Raw, etc., don't contain variables.
		}
	}
}

/// Recursively walks through an expression to find the root identifiers.
fn walk_expr(expr: &Expr, deps: &mut HashSet<String>) {
	// First, walk the main expression value
	walk_expr_val(&expr.val, deps);
	// Then, walk the expressions in any filters applied to it
	for filter in &expr.filters {
		walk_function_call(filter, deps);
	}
}

/// The core logic that processes an `ExprVal` enum.
fn walk_expr_val(expr_val: &ExprVal, deps: &mut HashSet<String>) {
	match expr_val {
		// This is the base case. We found an identifier.
		ExprVal::Ident(ident) => {
			// An identifier can be `user.name`. We only care about the root, `user`.
			if let Some(root) = ident.split('.').next() {
				deps.insert(root.to_string());
			}
		}
		// Recursive cases for complex expressions:
		ExprVal::Math(math_expr) => {
			walk_expr(&math_expr.lhs, deps);
			walk_expr(&math_expr.rhs, deps);
		}
		ExprVal::Logic(logic_expr) => {
			walk_expr(&logic_expr.lhs, deps);
			walk_expr(&logic_expr.rhs, deps);
		}
		ExprVal::FunctionCall(fn_call) => {
			walk_function_call(fn_call, deps);
		}
		ExprVal::Test(test) => {
			if let Some(root) = test.ident.split('.').next() {
				deps.insert(root.to_string());
			}
			for arg in &test.args {
				walk_expr(arg, deps);
			}
		}
		ExprVal::Array(arr) => {
			for item in arr {
				walk_expr(item, deps);
			}
		}
		ExprVal::StringConcat(sc) => {
			for val in &sc.values {
				walk_expr_val(val, deps);
			}
		}
		ExprVal::In(in_expr) => {
			walk_expr(&in_expr.lhs, deps);
			walk_expr(&in_expr.rhs, deps);
		}
		// Literals like String, Int, Float, Bool do not contain variables.
		_ => (),
	}
}

/// A helper to walk through the arguments of a function call or filter.
fn walk_function_call(call: &FunctionCall, deps: &mut HashSet<String>) {
	for arg_expr in call.args.values() {
		walk_expr(arg_expr, deps);
	}
}

impl PartialEq for Template {
	fn eq(&self, other: &Self) -> bool {
		self.input == other.input
	}
}

impl Eq for Template {}

impl<'de> Deserialize<'de> for Template {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		// Expect the input to be a simple string
		let text = String::deserialize(deserializer)?;
		Ok(Template::from(text))
	}
}

impl From<Template> for String {
	fn from(val: Template) -> Self {
		val.input
	}
}

impl<T: AsRef<str>> From<T> for Template {
	fn from(val: T) -> Self {
		let id = Uuid::new_v4().to_string();
		let input = val.as_ref().to_string();
		let template = tera::Template::new(&id, None, &input).unwrap();
		let dependencies = Self::get_dependencies(&template);
		Template { input, id, dependencies }
	}
}

#[cfg(test)]
mod tests {

	use super::*;
	use serde_test::{assert_de_tokens, Token};

	#[test]
	fn test_ser_de_empty() {
		let string = "{{ root }}";
		let template = Template::from(string);

		assert_de_tokens(&template, &[Token::String(string)]);
	}
}
