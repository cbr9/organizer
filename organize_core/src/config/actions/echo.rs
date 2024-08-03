use std::path::{Path, PathBuf};

use derive_more::Deref;
use serde::Deserialize;
use tera::Tera;

use crate::{config::actions::ActionType, path::get_context};
use anyhow::Result;

use super::ActionPipeline;

#[derive(Debug, Clone, Deserialize, Deref, Default, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Echo {
	message: String,
}

impl ActionPipeline for Echo {
	const REQUIRES_DEST: bool = false;
	const TYPE: ActionType = ActionType::Echo;

	fn execute<T: AsRef<Path> + Into<PathBuf> + Clone, P: AsRef<Path> + Into<PathBuf> + Clone>(
		&self,
		src: T,
		_: Option<P>,
		_: bool,
	) -> Result<Option<PathBuf>> {
		Ok(Some(src.into()))
	}

	fn log_success_msg<T: AsRef<Path> + Into<PathBuf> + Clone, P: AsRef<Path> + Into<PathBuf> + Clone>(
		&self,
		src: T,
		_: Option<P>,
		simulated: bool,
	) -> Result<String> {
		let context = get_context(&src);
		let message = Tera::one_off(&self.message, &context, false)?;
		let hint = if !simulated { "ECHO" } else { "SIMULATED ECHO" };
		Ok(format!("({}) {}", hint, message))
	}
}
