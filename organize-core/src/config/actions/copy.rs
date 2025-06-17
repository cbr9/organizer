use std::path::PathBuf;

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};

use crate::{
	config::{actions::common::enabled, context::ExecutionContext},
	path::prepare::prepare_target_path,
	resource::Resource,
	templates::template::Template,
};

use super::{common::ConflictResolution, Action};

#[derive(Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Copy {
	to: Template,
	#[serde(default)]
	if_exists: ConflictResolution,
	#[serde(default)]
	continue_with: ContinueWith,
	#[serde(default = "enabled")]
	pub enabled: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
enum ContinueWith {
	Original,
	Copy,
}

impl Default for ContinueWith {
	fn default() -> Self {
		Self::Copy
	}
}

#[typetag::serde(name = "copy")]
impl Action for Copy {
	fn templates(&self) -> Vec<&Template> {
		vec![&self.to]
	}

	#[tracing::instrument(ret(level = "info"), err(Debug), level = "debug", skip(ctx))]
	fn execute(&self, res: &Resource, ctx: &ExecutionContext) -> Result<Option<PathBuf>> {
		match prepare_target_path(&self.if_exists, res, &self.to, true, ctx)? {
			Some(reservation) => {
				if !ctx.settings.dry_run && self.enabled {
					std::fs::copy(res.path(), &reservation.path)
						.with_context(|| format!("Could not copy {} -> {}", res.path().display(), reservation.path.display()))?;
				}
				if self.continue_with == ContinueWith::Copy {
					Ok(Some(reservation.path))
				} else {
					Ok(Some(res.path().to_path_buf()))
				}
			}
			None => Ok(None),
		}
	}
}
