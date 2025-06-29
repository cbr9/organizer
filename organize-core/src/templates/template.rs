use std::{collections::HashMap, str::FromStr};

use chumsky::prelude::*;
use logos::Logos;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{error::Category, json}; // Import serde_json::Value and Map

use crate::{
	context::ExecutionContext,
	errors::Error,
	parser::{
		ast::{Expression, Segment, AST},
		errors::ParseError,
		lexer::{LexingError, Token},
		parser,
	},
	templates::{engine::TemplateError, prelude::Variable},
};

#[derive(Debug, PartialEq, Clone, Eq)]
pub enum Piece {
	Literal(String),
	Variable(Box<dyn Variable>),
}

#[derive(Debug, Serialize, Eq, PartialEq, Clone)]
pub struct Template {
	pub text: String,
	#[serde(skip)]
	pub pieces: Vec<Piece>,
}

impl std::ops::Deref for Template {
	type Target = Vec<Piece>;

	fn deref(&self) -> &Self::Target {
		&self.pieces
	}
}

impl<'de> Deserialize<'de> for Template {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		// Expect the input to be a simple string
		let text = String::deserialize(deserializer)?;
		Template::from_str(&text).map_err(|e| serde::de::Error::custom(e.to_string()))
	}
}

impl Template {
	fn parse(s: &str) -> Result<AST, ParseError> {
		let mut segments = Vec::new();
		let mut input = s;

		while !input.is_empty() {
			// Find the start of the next expression
			if let Some(start_delim) = input.find("{{") {
				// Anything before the `{{` is a literal
				if start_delim > 0 {
					segments.push(Segment::Literal(input[..start_delim].to_string()));
				}

				// Find the end of the expression
				let expr_content_start = start_delim + 2;
				if let Some(end_delim_relative) = input[expr_content_start..].find("}}") {
					let expr_content_end = expr_content_start + end_delim_relative;
					let expression_content = &input[expr_content_start..expr_content_end];

					// Lex and parse the content inside the delimiters
					let tokens = Token::lexer(expression_content)
						.collect::<Result<Vec<Token>, LexingError>>()
						.map_err(ParseError::LexingError)?;

					let (expression, errs) = parser().parse(&tokens).into_output_errors();

					match expression {
						Some(expr) => segments.push(Segment::Expression(expr)),
						None => {
							return Err(ParseError::InvalidExpression {
								content: format!("Could not parse expression: '{{ {expression_content} }}'. Errors: {errs:?}"),
							})
						}
					}

					// Advance the input slice past the `}}`
					input = &input[expr_content_end + 2..];
				} else {
					return Err(ParseError::MismatchedDelimiters {
						position: expr_content_start,
					});
				}
			} else {
				// No more `{{` found, the rest of the string is a literal
				segments.push(Segment::Literal(input.to_string()));
				break;
			}
		}

		Ok(AST { segments })
	}

	pub async fn render(&self, ctx: &ExecutionContext<'_>) -> Result<String, Error> {
		let mut rendered = vec![];
		for piece in self.pieces.iter() {
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

#[derive(Debug, Serialize)]
#[serde(untagged)] // Add the untagged attribute
enum Value {
	String(String),
	Map(HashMap<String, Value>),
}

// The main conversion function
fn convert_template_path_to_variable_arg_json(path_segments: &[String]) -> Option<serde_json::Value> {
	if path_segments.is_empty() {
		// No arguments provided (e.g., `{{ sys }}`), so Args should be None.
		return None;
	}

	if path_segments.len() == 1 {
		return Some(serde_json::to_value(&path_segments[0]).unwrap());
	}

	// The first segment is the top-level variant key (e.g., "os", "host", "path").
	let top_level_key = path_segments[0].clone();

	// Determine the value associated with the top-level key.
	let inner_value_for_variant = if path_segments.len() > 1 {
		// If there's a second segment, it's the specific sub-argument (e.g., "name", "uptime").
		// We assume it's a string representation of an enum variant.
		Value::String(path_segments[1].clone())
	} else {
		// If only one segment (e.g., `["os"]`), it means use the default
		// for that top-level argument's sub-enum. We represent this as an empty JSON object `{}`.
		// The custom Deserialize implementation for the specific `Args` enum will then
		// interpret this empty object as the default variant.
		Value::Map(HashMap::new())
	};

	// Construct the JSON object: {"top_level_key": inner_value_for_variant}
	let mut map = HashMap::new();
	map.insert(top_level_key, inner_value_for_variant);
	Some(serde_json::to_value(map).unwrap()) // Convert HashMap to serde_json::Map for Value::Object
}

impl FromStr for Template {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let text = s.to_string();
		let ast = Self::parse(s)?;

		let mut segments = vec![];

		for segment in ast.segments.iter() {
			match segment {
				Segment::Literal(literal) => {
					segments.push(Piece::Literal(literal.clone()));
				}
				Segment::Expression(expression) => match expression {
					Expression::Variable(parts) => {
						let mut fields = parts.clone();
						let var_name = fields.remove(0);
						let maybe_fields = convert_template_path_to_variable_arg_json(fields.as_slice());

						let json_input = json!({
							"type": var_name,
							"value": maybe_fields
						});

						let variable: Box<dyn Variable> = match serde_json::from_value(json_input) {
							Ok(variable) => variable,
							Err(e) => {
								tracing::error!("{}", e);
								return Err(Error::TemplateError(TemplateError::DeserializationError {
									source: e,
									variable: var_name,
									fields: fields,
								}));
							}
						};
						segments.push(Piece::Variable(variable));
					}
				},
			}
		}
		Ok(Self { text, pieces: segments })
	}
}

// #[cfg(test)]
// mod tests {

// 	use crate::parser::ast::Expression;

// 	use super::*; // Import your parser function and AST structs

// 	#[test]
// 	fn test_parse_simple_variable() {
// 		let template = Template::from_str("{{ name }}").unwrap();
// 		let expected = AST {
// 			segments: vec![Segment::Expression(Expression::Variable(VariablePath {
// 				parts: vec!["name".to_string()],
// 			}))],
// 		};
// 		assert_eq!(template.ast, expected);
// 	}

// 	#[test]
// 	fn test_parse_nested_path() {
// 		let template = Template::from_str("{{ user.address.city }}").unwrap();
// 		let expected = AST {
// 			segments: vec![Segment::Expression(Expression::Variable(VariablePath {
// 				parts: vec!["user".to_string(), "address".to_string(), "city".to_string()],
// 			}))],
// 		};
// 		assert_eq!(template.ast, expected);
// 	}

// 	#[test]
// 	fn test_parse_mixed_content() {
// 		let template = Template::from_str("Hello, {{ user.name }}!").unwrap();
// 		let expected = AST {
// 			segments: vec![
// 				Segment::Literal("Hello, ".to_string()),
// 				Segment::Expression(Expression::Variable(VariablePath {
// 					parts: vec!["user".to_string(), "name".to_string()],
// 				})),
// 				Segment::Literal("!".to_string()),
// 			],
// 		};
// 		assert_eq!(template.ast, expected);
// 	}
// 	#[test]
// 	fn test_parse_mixed_content2() {
// 		let template = Template::from_str("Hello, {{ user.name }}! {{ user.name }}").unwrap();
// 		let expected = AST {
// 			segments: vec![
// 				Segment::Literal("Hello, ".to_string()),
// 				Segment::Expression(Expression::Variable(VariablePath {
// 					parts: vec!["user".to_string(), "name".to_string()],
// 				})),
// 				Segment::Literal("! ".to_string()),
// 				Segment::Expression(Expression::Variable(VariablePath {
// 					parts: vec!["user".to_string(), "name".to_string()],
// 				})),
// 			],
// 		};
// 		assert_eq!(template.ast, expected);
// 	}
// 	#[test]
// 	fn test_parse_mixed_content3() {
// 		let template = Template::from_str("{{ user.name }} Hello, {{ user.name }}! {{ user.name }}").unwrap();
// 		let expected = AST {
// 			segments: vec![
// 				Segment::Expression(Expression::Variable(VariablePath {
// 					parts: vec!["user".to_string(), "name".to_string()],
// 				})),
// 				Segment::Literal(" Hello, ".to_string()),
// 				Segment::Expression(Expression::Variable(VariablePath {
// 					parts: vec!["user".to_string(), "name".to_string()],
// 				})),
// 				Segment::Literal("! ".to_string()),
// 				Segment::Expression(Expression::Variable(VariablePath {
// 					parts: vec!["user".to_string(), "name".to_string()],
// 				})),
// 			],
// 		};
// 		assert_eq!(template.ast, expected);
// 	}

// 	#[test]
// 	fn test_parse_syntax_error() {
// 		// A path cannot end with a dot
// 		let result = Template::from_str("{{ user.name. }}");
// 		assert!(result.is_err_and(|x| x.is_invalid_expression()));
// 	}

// 	#[test]
// 	fn test_mismatched_delimiters() {
// 		let result = Template::from_str("{{ user.name");
// 		assert!(result.is_err_and(|x| x.is_mismatched_delimiters()));
// 	}
// 	#[test]
// 	fn test_lexing_error() {
// 		let result = Template::from_str("{{ user.@name }}");
// 		dbg!(&result);
// 		assert!(result.is_err_and(|x| x.is_lexing_error()));
// 	}
// }
