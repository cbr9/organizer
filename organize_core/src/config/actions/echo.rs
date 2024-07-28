use std::path::{Path, PathBuf};

use derive_more::Deref;
use serde::Deserialize;

use crate::{config::actions::ActionType, string::ExpandPlaceholder};
use anyhow::Result;

use super::ActionPipeline;

#[derive(Debug, Clone, Deserialize, Deref, Default, Eq, PartialEq)]
pub struct Echo {
	message: String,
}

impl ActionPipeline for Echo {
	const TYPE: ActionType = ActionType::Echo;
	const REQUIRES_DEST: bool = false;

	fn execute<T: AsRef<Path> + Into<PathBuf> + Clone, P: AsRef<Path> + Into<PathBuf> + Clone>(
		&self,
		src: T,
		_: Option<P>,
	) -> Result<Option<PathBuf>> {
		Ok(Some(src.into()))
	}

	fn log_success_msg<T: AsRef<Path> + Into<PathBuf> + Clone, P: AsRef<Path> + Into<PathBuf> + Clone>(
		&self,
		src: T,
		_: Option<P>,
	) -> Result<String> {
		let expanded = self.message.as_str().expand_placeholders(src)?;
		Ok(format!("(ECHO) {}", expanded.to_string_lossy()))
	}
}
