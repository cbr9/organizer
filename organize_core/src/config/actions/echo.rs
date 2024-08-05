use std::{
	path::{Path, PathBuf},
};

use derive_more::Deref;
use serde::Deserialize;

use crate::{
	config::{actions::ActionType, SIMULATION},
	resource::Resource,
	templates::TERA,
};
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

	fn execute<T: AsRef<Path>>(&self, src: &mut Resource, _: Option<T>) -> Result<Option<PathBuf>> {
		Ok(Some(src.path().into_owned()))
	}

	fn log_success_msg<T: AsRef<Path>>(&self, src: &mut Resource, _: Option<T>) -> Result<String> {
		let message = TERA.lock().unwrap().render_str(&self.message, &src.context())?;
		let hint = if !*SIMULATION { "ECHO" } else { "SIMULATED ECHO" };
		Ok(format!("({}) {}", hint, message))
	}
}
