use std::path::{Path, PathBuf};

use derive_more::Deref;
use serde::Deserialize;

use crate::{config::actions::ActionType, resource::Resource, templates::TERA};
use anyhow::Result;

use super::AsAction;

#[derive(Debug, Clone, Deserialize, Deref, Default, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Echo {
	message: String,
}

impl AsAction for Echo {
	const REQUIRES_DEST: bool = false;
	const TYPE: ActionType = ActionType::Echo;

	fn execute<T: AsRef<Path>>(&self, src: &Resource, _: Option<T>, _: bool) -> Result<Option<PathBuf>> {
		Ok(Some(src.path.clone()))
	}

	fn log_success_msg<T: AsRef<Path>>(&self, src: &Resource, _: Option<&T>, dry_run: bool) -> Result<String> {
		let message = TERA.lock().unwrap().render_str(&self.message, &src.context)?;
		let hint = if !dry_run { "ECHO" } else { "SIMULATED ECHO" };
		Ok(format!("({}) {}", hint, message))
	}
}
