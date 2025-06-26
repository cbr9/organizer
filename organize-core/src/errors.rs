use std::path::PathBuf;
use thiserror::Error;

use crate::{action::UndoError, templates::engine::TemplateError};

/// The primary error type for all actions within the application.
#[derive(Error, Debug)]
pub enum Error {
	#[error("Error converting to value")]
	Json(#[from] serde_json::Error),

	#[error(transparent)]
	Io(#[from] std::io::Error),

	#[error("Could not create backup for: {path:?}")]
	Backup {
		#[source]
		source: std::io::Error,
		path: PathBuf,
	},

	#[error("invalid path")]
	InvalidPath { path: PathBuf },

	#[error("Could not resolve path from template: '{template}'")]
	PathResolution { template: String },

	#[error("Error in prompt")]
	Interaction {
		#[source]
		source: std::io::Error,
		prompt: String,
	},

	#[error("Could not render template")]
	TemplateError(#[from] TemplateError),

	#[error("Tried to retrieve `{0}` from the scope but it is not defined")]
	ScopeError(String),

	#[error(transparent)]
	UndoError(#[from] UndoError),
}
