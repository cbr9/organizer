use std::path::{Path, PathBuf};

use derive_more::Deref;
use serde::Deserialize;
use tera::Tera;

use crate::{config::actions::ActionType, path::get_context};
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
		let context = get_context(&src);
		let message = Tera::one_off(&self.message, &context, false)?;
		Ok(format!("(ECHO) {}", message))
	}
}
