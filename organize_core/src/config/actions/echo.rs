use std::path::{Path, PathBuf};

use derive_more::Deref;
use serde::Deserialize;

use crate::{resource::Resource, templates::Template};
use anyhow::Result;

use super::{script::ActionConfig, AsAction};

#[derive(Debug, Clone, Deserialize, Deref, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Echo {
	message: Template,
}

impl AsAction for Echo {
	const CONFIG: ActionConfig = ActionConfig {
		requires_dest: false,
		parallelize: true,
	};

	#[tracing::instrument(ret(level = "info"), err(Debug), level = "debug", skip(_dest, _dry_run))]
	fn execute<T: AsRef<Path>>(&self, src: &Resource, _dest: Option<T>, _dry_run: bool) -> Result<Option<PathBuf>> {
		let message = self.message.render(&src.context).map_err(anyhow::Error::msg)?;
		tracing::info!("{}", message);
		Ok(Some(src.path.clone()))
	}
}
