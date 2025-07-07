use pest::{iterators::Pair, Parser};
use pest_derive::Parser;
use thiserror::Error;

// The AST definitions are moved here. They remain unchanged.
#[derive(Debug, PartialEq, Clone, Eq)]
pub enum Expression {
	Variable(Vec<String>),
	Literal(String), // For arguments
	FunctionCall { name: String, args: Vec<Expression> },
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

fn build_ast_from_expression(pair: Pair<Rule>) -> Expression {
	match pair.as_rule() {
		// Handle wrapper rules by descending into their inner content.
		Rule::expression | Rule::argument => build_ast_from_expression(pair.into_inner().next().unwrap()),

		Rule::variable => {
			let parts = pair.into_inner().map(|p| p.as_str().to_string()).collect();
			Expression::Variable(parts)
		}
		Rule::function_call => {
			let mut inner = pair.into_inner();
			let name = inner.next().unwrap().as_str().to_string();
			let args = inner.map(build_ast_from_expression).collect();
			Expression::FunctionCall { name, args }
		}
		// A literal_string is a choice, so we descend into the actual quoted string rule.
		Rule::literal_string => build_ast_from_expression(pair.into_inner().next().unwrap()),

		// The rules for the quoted strings themselves. Here we extract the content.
		Rule::single_quoted_string | Rule::double_quoted_string => {
			// The inner pair is the content between the quotes.
			// It's optional to handle empty strings like "" or ''.
			let content = pair.into_inner().next().map(|p| p.as_str()).unwrap_or("");
			Expression::Literal(content.to_string())
		}
		rule => unreachable!("build_ast_from_expression was called with an unexpected rule: {:?}", rule),
	}
}

impl AST {
	pub fn parse(s: &str) -> Result<Self, ParseError> {
		if s.is_empty() {
			return Ok(AST { segments: vec![] });
		}
		let pairs = PestParser::parse(Rule::template, s)?.next().unwrap().into_inner();
		let mut segments = Vec::new();

		for pair in pairs {
			match pair.as_rule() {
				Rule::literal => {
					segments.push(Segment::Literal(pair.as_str().to_string()));
				}
				Rule::moustache => {
					let inner_expr_pair = pair.into_inner().next().unwrap();
					let expression = build_ast_from_expression(inner_expr_pair);
					segments.push(Segment::Expression(expression));
				}
				Rule::EOI => (),
				rule => unreachable!("Unexpected top-level rule: {:?}", rule),
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
