use std::path::PathBuf;

use derive_more::Deref;
use serde::{Deserialize, Serialize};

use crate::{resource::Resource, templates::Template};
use anyhow::Result;

use super::{script::ActionConfig, Action};

#[derive(Debug, Clone, Deserialize, Serialize, Deref, Default, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Echo {
	message: Template,
}

#[typetag::serde(name = "echo")]
impl Action for Echo {
	fn config(&self) -> ActionConfig {
		ActionConfig {
			requires_dest: false,
			parallelize: true,
		}
	}

	#[tracing::instrument(ret(level = "info"), err(Debug), level = "debug", skip(_dest, _dry_run))]
	fn execute(&self, src: &Resource, _dest: Option<PathBuf>, _dry_run: bool) -> Result<Option<PathBuf>> {
		let message = self.message.render(&src.context).map_err(anyhow::Error::msg)?;
		tracing::info!("{}", message);
		Ok(Some(src.path.clone()))
	}
}
