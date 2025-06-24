use std::{char::ParseCharError, convert::Infallible, str::FromStr};

use logos::{Lexer, Logos};
use thiserror::Error;

#[derive(Error, Default, Debug, Clone, PartialEq)]
pub enum LexingError {
	#[error("Invalid identifier: {0}")]
	InvalidIdentifier(String),
	#[default]
	#[error("Other error")]
	Other,
}

impl From<anyhow::Error> for LexingError {
	fn from(value: anyhow::Error) -> Self {
		LexingError::InvalidIdentifier(value.to_string())
	}
}
impl From<&str> for LexingError {
	fn from(value: &str) -> Self {
		LexingError::InvalidIdentifier(value.to_string())
	}
}

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\f\n]+", error = LexingError)]
pub enum Token<'a> {
	#[token("{{")]
	OpenDelim,
	#[token("}}")]
	CloseDelim,
	#[token(".")]
	Dot,
	#[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", check_valid_ident)]
	Identifier(&'a str),
}

fn check_valid_ident<'a>(lex: &mut logos::Lexer<'a, Token<'a>>) -> Result<&'a str, anyhow::Error> {
	let regex = regex::Regex::new(r"[a-zA-Z_][a-zA-Z0-9_]*").unwrap();
	let input = lex.slice();
	if regex.is_match(input) {
		Ok(input)
	} else {
		Err(anyhow::Error::msg(input.to_string()))
	}
}

impl<'a> std::fmt::Display for Token<'a> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::OpenDelim => write!(f, "{{"),
			Self::CloseDelim => write!(f, "}}"),
			Self::Dot => write!(f, "."),
			Self::Identifier(word) => write!(f, "{word}"),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*; // Import Token and Logos

	fn lex(input: &str) -> Vec<Result<Token, LexingError>> {
		// A helper function to collect all tokens from the lexer
		Token::lexer(input).collect()
	}

	#[test]
	fn test_simple_variable() {
		let tokens = lex("{{ name }}");
		assert_eq!(tokens, vec![
			Ok(Token::OpenDelim),
			Ok(Token::Identifier("name".into())),
			Ok(Token::CloseDelim)
		]);
	}

	#[test]
	fn test_path_expression() {
		let tokens = lex("{{ user.address.city }}");
		assert_eq!(tokens, vec![
			Ok(Token::OpenDelim),
			Ok(Token::Identifier("user".into())),
			Ok(Token::Dot),
			Ok(Token::Identifier("address".into())),
			Ok(Token::Dot),
			Ok(Token::Identifier("city".into())),
			Ok(Token::CloseDelim)
		]);
	}

	#[test]
	fn test_invalid_token() {
		// Logos will produce an Error token for characters it doesn't recognize
		// In our current lexer, most symbols would be skipped or cause an error.
		// Let's assume we modify the lexer slightly to capture errors.
		// For now, this test shows that it simply ignores non-defined tokens.
		let tokens = lex("{{ user.name! }}");
		dbg!(&tokens);
		assert_eq!(tokens, vec![
			Ok(Token::OpenDelim),
			Ok(Token::Identifier("user".into())),
			Ok(Token::Dot),
			Ok(Token::Identifier("name".into())),
			Err(LexingError::InvalidIdentifier("!".into())),
			Ok(Token::CloseDelim),
		]);
	}

	#[test]
	fn lexer_handles_invalid_character_within_expression() {
		// Logos will produce an Error token for characters it doesn't recognize
		// In our current lexer, most symbols would be skipped or cause an error.
		// Let's assume we modify the lexer slightly to capture errors.
		// For now, this test shows that it simply ignores non-defined tokens.
		let tokens = lex("{{ path! }}");
		dbg!(&tokens);
		assert_eq!(tokens, vec![
			Ok(Token::OpenDelim),
			Ok(Token::Identifier("path".into())),
			Err(LexingError::InvalidIdentifier("!".into())),
			Ok(Token::CloseDelim),
		]);
	}

	#[test]
	fn lexer_handles_invalid_start_of_identifier() {
		// Identifiers in our language cannot start with a number.
		let tokens = lex("{{ 1st_place }}");
		assert_eq!(tokens, vec![
			Ok(Token::OpenDelim),
			Err(LexingError::InvalidIdentifier("1".into())),
			Ok(Token::Identifier("st_place".into())),
			Ok(Token::CloseDelim)
		])
	}

	#[test]
	fn lexer_handles_multiple_errors() {
		// Identifiers in our language cannot start with a number.
		let tokens = lex("{{ user.@email + }}");
		assert_eq!(tokens, vec![
			Ok(Token::OpenDelim),
			Ok(Token::Identifier("user".into())),
			Ok(Token::Dot),
			Err(LexingError::InvalidIdentifier("@".into())),
			Ok(Token::Identifier("email".into())),
			Err(LexingError::InvalidIdentifier("+".into())),
			Ok(Token::CloseDelim)
		])
	}

	#[test]
	fn lexer_handles_empty_expression() {
		let tokens = lex("{{}}");
		assert_eq!(tokens, vec![Ok(Token::OpenDelim), Ok(Token::CloseDelim)])
	}
}
