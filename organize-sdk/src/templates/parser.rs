use pest::{iterators::Pair, Parser};
use pest_derive::Parser;
use thiserror::Error;

// The AST definitions are moved here. They remain unchanged.
#[derive(Debug, PartialEq, Clone, Eq)]
pub enum Expression {
	Variable(Vec<String>),
}

#[derive(Debug, PartialEq, Clone, Eq)]
pub enum Segment {
	Literal(String),
	Expression(Expression),
}

#[derive(Debug, PartialEq, Clone, Eq)]
pub struct AST {
	pub segments: Vec<Segment>,
}

// A new, simpler error enum for the pest parser.
#[derive(Error, Debug)]
pub enum ParseError {
	#[error("Mismatched delimiters: found '{{' with no closing '}}'")]
	MismatchedDelimiters,
	#[error("Pest parser error: {0}")]
	Pest(#[from] Box<pest::error::Error<Rule>>),
}

// This struct links our code to the grammar file.
#[derive(Parser)]
#[grammar = "templates/grammar.pest"]
struct PestParser;

fn build_expression_ast(pair: Pair<Rule>) -> Expression {
	match pair.as_rule() {
		Rule::variable => {
			let parts = pair.into_inner().map(|p| p.as_str().to_string()).collect();
			Expression::Variable(parts)
		}
		_ => unreachable!("build_expression_ast expects an expression rule, found {:?}", pair.as_rule()),
	}
}

impl AST {
	/// Parses the entire template string using the pest grammar.
	/// This method now delegates all parsing to pest, which handles the
	/// entire structure of the template, including literals and delimiters.
	pub fn parse(s: &str) -> Result<Self, ParseError> {
		// If the input is empty, return an empty AST.
		if s.is_empty() {
			return Ok(AST { segments: vec![] });
		}

		// Parse the entire string with the top-level `template` rule.
		// This gives us an iterator directly over `literal` and `delimited_expression` pairs.
		let pairs = PestParser::parse(Rule::template, s)?.next().unwrap().into_inner();
		let mut segments = Vec::new();

		for pair in pairs {
			match pair.as_rule() {
				Rule::literal => {
					segments.push(Segment::Literal(pair.as_str().to_string()));
				}
				Rule::moustache => {
					// Go inside `{{...}}` to get the actual `expression`.
					let inner_expr_pair = pair.into_inner().next().unwrap();
					let expression = build_expression_ast(inner_expr_pair);
					segments.push(Segment::Expression(expression));
				}
				Rule::EOI => (), // End-of-input is expected, do nothing.
				_ => unreachable!("Unexpected top-level rule: {:?}", pair.as_rule()),
			}
		}

		Ok(AST { segments })
	}
}

// Helper to convert pest errors into our custom ParseError.
impl From<pest::error::Error<Rule>> for ParseError {
	fn from(error: pest::error::Error<Rule>) -> Self {
		ParseError::Pest(Box::new(error))
	}
}
