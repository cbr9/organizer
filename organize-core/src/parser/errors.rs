use crate::parser::lexer::LexingError;
use thiserror::Error;

// This will be the single, unified error type returned by your parser.
#[derive(Error, Debug)]
pub enum ParseError {
	#[error("Mismatched delimiters: found '{{' at position {position} with no closing '}}'")]
	MismatchedDelimiters { position: usize },

	#[error("Could not parse expression: '{{ {content} }}'")]
	InvalidExpression { content: String },

	#[error(transparent)]
	LexingError(#[from] LexingError),
}

impl ParseError {
	/// Returns `true` if the parse error is [`MismatchedDelimiters`].
	///
	/// [`MismatchedDelimiters`]: ParseError::MismatchedDelimiters
	#[must_use]
	pub fn is_mismatched_delimiters(&self) -> bool {
		matches!(self, Self::MismatchedDelimiters { .. })
	}

	/// Returns `true` if the parse error is [`InvalidExpression`].
	///
	/// [`InvalidExpression`]: ParseError::InvalidExpression
	#[must_use]
	pub fn is_invalid_expression(&self) -> bool {
		matches!(self, Self::InvalidExpression { .. })
	}

	/// Returns `true` if the parse error is [`LexingError`].
	///
	/// [`LexingError`]: ParseError::LexingError
	#[must_use]
	pub fn is_lexing_error(&self) -> bool {
		matches!(self, Self::LexingError { .. })
	}
}
