use chumsky::prelude::*;

use crate::parser::{
	ast::{Expression, VariablePath},
	lexer::Token,
};

pub mod ast;
pub mod errors;
pub mod lexer;

pub fn parser<'a>() -> impl Parser<'a, &'a [Token<'a>], Expression, extra::Err<Simple<'a, Token<'a>>>> {
	// A parser for a single identifier token
	let ident = select! { Token::Identifier(s) => s.to_string() };

	// A parser for a dot-separated path of one or more identifiers
	let var_path = ident
		.separated_by(just(Token::Dot))
		.at_least(1) // must have at least one part
		.collect::<Vec<String>>()
		.map(|parts| VariablePath { parts });

	var_path.map(Expression::Variable)
}
