use std::path::PathBuf;
use thiserror::Error;

use crate::{
	plugins::action::UndoError,
	templates::{engine::TemplateError, parser::ParseError},
};

/// The primary error type for all actions within the application.
#[derive(Error, Debug)]
pub enum Error {
	#[error(transparent)]
	SFTP(#[from] russh_sftp::client::error::Error),

	#[error(transparent)]
	SSH(#[from] russh::Error),

	#[error("Impossible operation: {0}")]
	ImpossibleOp(String),

	#[error("Error in configuration: {0}")]
	Config(String),

	#[error("Error converting to value")]
	Json(#[from] serde_json::Error),

	#[error(transparent)]
	Io(#[from] std::io::Error),

	#[error(transparent)]
	Other(#[from] anyhow::Error),

	#[error("Could not create backup for: {path:?}")]
	Backup {
		#[source]
		source: std::io::Error,
		path: PathBuf,
	},

	#[error(transparent)]
	ParseError(#[from] ParseError),

	#[error("Error in prompt")]
	Interaction {
		#[source]
		source: std::io::Error,
		prompt: String,
	},

	#[error(transparent)]
	TemplateError(#[from] TemplateError),

	#[error("Tried to retrieve `{0}` from the scope but it is not defined")]
	OutOfScope(String),

	#[error(transparent)]
	UndoError(#[from] UndoError),
}
