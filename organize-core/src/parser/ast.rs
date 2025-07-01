use chumsky::Parser;
use logos::Logos;

use crate::parser::{
	errors::ParseError,
	lexer::{LexingError, Token},
	parser,
};

// For now, an Expression is just a VariablePath.
// We can add Literals here later if needed.
#[derive(Debug, PartialEq, Clone, Eq)]
pub enum Expression {
	Variable(Vec<String>),
}

// A full template is a sequence of literal text and expressions.
#[derive(Debug, PartialEq, Clone, Eq)]
pub enum Segment {
	Literal(String),
	Expression(Expression),
}

#[derive(Debug, PartialEq, Clone, Eq)]
pub struct AST {
	pub segments: Vec<Segment>,
}

impl AST {
	/// Parses a raw string into an Abstract Syntax Tree (AST).
	/// This function orchestrates the lexer and expression parser.
	pub fn parse(s: &str) -> Result<Self, ParseError> {
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
					let expression_content = &input[expr_content_start..expr_content_end].trim();

					// Lex and parse the content inside the delimiters
					let tokens = Token::lexer(expression_content).collect::<Result<Vec<Token<'_>>, LexingError>>()?;

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
