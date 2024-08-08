use std::path::{Path, PathBuf};

use derive_more::Deref;
use serde::Deserialize;

use crate::{
	resource::Resource,
	templates::{Template},
};
use anyhow::Result;

use super::{script::ActionConfig, AsAction};

#[derive(Debug, Clone, Deserialize, Deref, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Echo {
	message: Template,
}

impl<'a> AsAction<'a> for Echo {
	fn execute<T: AsRef<Path>>(&self, src: &Resource, _: Option<T>, _: bool) -> Result<Option<PathBuf>> {
		Ok(Some(src.path.clone()))
	}

	fn log_message<T: AsRef<Path>>(&self, src: &Resource, _: Option<&T>, _: bool) -> Result<String> {
		self.message.expand(&src.context).map_err(anyhow::Error::msg)
	}

	const CONFIG: ActionConfig<'a> = ActionConfig {
		requires_dest: false,
		log_hint: "ECHO",
	};
}
