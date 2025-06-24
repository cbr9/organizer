use std::{char::ParseCharError, convert::Infallible, str::FromStr};

use anyhow::Result;
use logos::{Lexer, Logos};
use thiserror::Error;

#[derive(Error, Default, Debug, Clone, PartialEq)]
pub enum LexingError {
	#[error("Invalid identifier: {0}")]
	InvalidIdentifier(String),
	#[default]
	#[error("Unknown lexing error")]
	Other,
}

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\f\n]+", error = LexingError, extras = (usize, usize))]
pub enum Token<'a> {
	#[token("{{")]
	OpenDelim,
	#[token("}}")]
	CloseDelim,
	#[token(".")]
	Dot,
	#[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
	Identifier(&'a str),
	#[regex(".",  |lex| Err(LexingError::InvalidIdentifier(lex.slice().to_string())), priority = 1)]
	InvalidToken(LexingError),
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
		// 	// Logos will produce an Error token for characters it doesn't recognize
		// 	// In our current lexer, most symbols would be skipped or cause an error.
		// 	// Let's assume we modify the lexer slightly to capture errors.
		// 	// For now, this test shows that it simply ignores non-defined tokens.
		let tokens = lex("{{ path! }}");
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
