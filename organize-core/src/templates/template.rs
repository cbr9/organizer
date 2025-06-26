use std::str::FromStr;

use chumsky::prelude::*;
use logos::Logos;
use serde::{Deserialize, Deserializer, Serialize};

use crate::parser::{
	ast::{Segment, AST},
	errors::ParseError,
	lexer::{LexingError, Token},
	parser,
};

#[derive(Debug, Serialize, Eq, PartialEq, Clone)]
pub struct Template {
	pub text: String,
	#[serde(skip)]
	pub ast: AST,
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
	type Err = ParseError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let text = s.to_string();
		let ast = Self::parse(s)?;
		Ok(Self { text, ast })
	}
}

#[cfg(test)]
mod tests {

	use crate::parser::ast::{Expression, VariablePath};

	use super::*; // Import your parser function and AST structs

	#[test]
	fn test_parse_simple_variable() {
		let template = Template::from_str("{{ name }}").unwrap();
		let expected = AST {
			segments: vec![Segment::Expression(Expression::Variable(VariablePath {
				parts: vec!["name".to_string()],
			}))],
		};
		assert_eq!(template.ast, expected);
	}

	#[test]
	fn test_parse_nested_path() {
		let template = Template::from_str("{{ user.address.city }}").unwrap();
		let expected = AST {
			segments: vec![Segment::Expression(Expression::Variable(VariablePath {
				parts: vec!["user".to_string(), "address".to_string(), "city".to_string()],
			}))],
		};
		assert_eq!(template.ast, expected);
	}

	#[test]
	fn test_parse_mixed_content() {
		let template = Template::from_str("Hello, {{ user.name }}!").unwrap();
		let expected = AST {
			segments: vec![
				Segment::Literal("Hello, ".to_string()),
				Segment::Expression(Expression::Variable(VariablePath {
					parts: vec!["user".to_string(), "name".to_string()],
				})),
				Segment::Literal("!".to_string()),
			],
		};
		assert_eq!(template.ast, expected);
	}
	#[test]
	fn test_parse_mixed_content2() {
		let template = Template::from_str("Hello, {{ user.name }}! {{ user.name }}").unwrap();
		let expected = AST {
			segments: vec![
				Segment::Literal("Hello, ".to_string()),
				Segment::Expression(Expression::Variable(VariablePath {
					parts: vec!["user".to_string(), "name".to_string()],
				})),
				Segment::Literal("! ".to_string()),
				Segment::Expression(Expression::Variable(VariablePath {
					parts: vec!["user".to_string(), "name".to_string()],
				})),
			],
		};
		assert_eq!(template.ast, expected);
	}
	#[test]
	fn test_parse_mixed_content3() {
		let template = Template::from_str("{{ user.name }} Hello, {{ user.name }}! {{ user.name }}").unwrap();
		let expected = AST {
			segments: vec![
				Segment::Expression(Expression::Variable(VariablePath {
					parts: vec!["user".to_string(), "name".to_string()],
				})),
				Segment::Literal(" Hello, ".to_string()),
				Segment::Expression(Expression::Variable(VariablePath {
					parts: vec!["user".to_string(), "name".to_string()],
				})),
				Segment::Literal("! ".to_string()),
				Segment::Expression(Expression::Variable(VariablePath {
					parts: vec!["user".to_string(), "name".to_string()],
				})),
			],
		};
		assert_eq!(template.ast, expected);
	}

	#[test]
	fn test_parse_syntax_error() {
		// A path cannot end with a dot
		let result = Template::from_str("{{ user.name. }}");
		assert!(result.is_err_and(|x| x.is_invalid_expression()));
	}

	#[test]
	fn test_mismatched_delimiters() {
		let result = Template::from_str("{{ user.name");
		assert!(result.is_err_and(|x| x.is_mismatched_delimiters()));
	}
	#[test]
	fn test_lexing_error() {
		let result = Template::from_str("{{ user.@name }}");
		dbg!(&result);
		assert!(result.is_err_and(|x| x.is_lexing_error()));
	}
}
