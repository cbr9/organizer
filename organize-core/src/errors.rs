use std::path::PathBuf;
use thiserror::Error;
use zip::result::ZipError;

use crate::{
	config::{
		// actions::email::EmailError,
		context::ExecutionScope,
	},
	templates::template::Template,
};

/// A self-contained, owned snapshot of the execution context at the time of an error.
/// It has no lifetimes, so it can be freely passed around.
#[derive(Debug, Clone)]
pub struct ErrorContext {
	pub rule_id: Option<String>,
	pub rule_index: usize,
	pub folder_path: PathBuf,
}

impl ErrorContext {
	pub fn from_scope(scope: &ExecutionScope) -> Self {
		Self {
			rule_id: scope.rule.id.clone(),
			rule_index: scope.rule.index,
			folder_path: scope.folder.path.clone(),
		}
	}
}

/// The primary error type for all actions within the application.
#[derive(Error, Debug)]
pub enum Error {
	#[error("I/O error for path: {path:?}")]
	Io {
		#[source]
		source: std::io::Error,
		path: PathBuf,
		target: Option<PathBuf>,
		context: ErrorContext,
	},

	#[error("Could not extract {path:?}")]
	Extraction {
		#[source]
		source: ZipError,
		path: PathBuf,
		context: ErrorContext,
	},

	#[error("Could not resolve path from template: '{template}'")]
	PathResolution { template: String, context: ErrorContext },

	#[error("Error in prompt")]
	Interaction {
		#[source]
		source: std::io::Error,
		prompt: String,
		context: ErrorContext,
	},

	#[error("Could not render template")]
	Template {
		#[source]
		source: tera::Error,
		template: Template,
		context: ErrorContext,
	},

	// #[error("Email action failed")]
	// Email {
	// 	#[source]
	// 	source: EmailError,
	// 	context: ErrorContext,
	// },
	#[error("Script crashed. Check the final script at {script}")]
	Script {
		#[source]
		source: std::io::Error,
		script: PathBuf,
		context: ErrorContext,
	},

	#[error("Could not send {path} to trash")]
	Trash {
		#[source]
		source: trash::Error,
		path: PathBuf,
		context: ErrorContext,
	},
}

impl Error {
	/// Helper method to consistently access the context from any error variant.
	pub fn context(&self) -> &ErrorContext {
		match self {
			Error::Io { context, .. } => context,
			Error::PathResolution { context, .. } => context,
			Error::Template { context, .. } => context,
			// Error::Email { context, .. } => context,
			Error::Extraction { context, .. } => context,
			Error::Interaction { context, .. } => context,
			Error::Script { context, .. } => context,
			Error::Trash { context, .. } => context,
		}
	}
}
