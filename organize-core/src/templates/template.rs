use std::str::FromStr;

use chumsky::prelude::*;
use logos::Logos;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::json;

use crate::{
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
pub enum BuiltSegment {
	Literal(String),
	Expression(Box<dyn Variable>, Vec<String>),
}

#[derive(Debug, Serialize, Eq, PartialEq, Clone)]
pub struct Template {
	pub text: String,
	#[serde(skip)]
	ast: AST,
	#[serde(skip)]
	pub variables: Vec<BuiltSegment>,
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
					segments.push(BuiltSegment::Literal(literal.clone()));
				}
				Segment::Expression(expression) => match expression {
					Expression::Variable(parts) => {
						let json_input = json!({
							"type": &parts[0],
						});
						let joined = format!(r#"{{{{ {} }}}}"#, parts.join("."));
						let variable: Box<dyn Variable> = serde_json::from_value(json_input).map_err(|_| TemplateError::UnknownVariable(joined))?;
						let mut fields = parts.clone();
						if parts.len() > 1 {
							fields.remove(0);
							segments.push(BuiltSegment::Expression(variable, fields));
						} else {
							segments.push(BuiltSegment::Expression(variable, vec![]));
						}
					}
				},
			}
		}
		Ok(Self {
			text,
			ast,
			variables: segments,
		})
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
