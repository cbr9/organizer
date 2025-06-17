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

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Hardlink {
	to: Template,
	#[serde(default)]
	if_exists: ConflictResolution,
	#[serde(default)]
	continue_with: ContinueWith,
	#[serde(default = "enabled")]
	enabled: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
enum ContinueWith {
	Original,
	Link,
}

impl Default for ContinueWith {
	fn default() -> Self {
		Self::Original
	}
}

#[typetag::serde(name = "hardlink")]
impl Action for Hardlink {
	fn templates(&self) -> Vec<&Template> {
		vec![&self.to]
	}

	#[tracing::instrument(ret(level = "info"), err(Debug), level = "debug", skip(ctx))]
	fn execute(&self, res: &Resource, ctx: &ExecutionContext) -> Result<Option<PathBuf>> {
		match prepare_target_path(&self.if_exists, res, &self.to, true, ctx)? {
			Some(reservation) => {
				if !ctx.settings.dry_run && self.enabled {
					std::fs::hard_link(res.path(), &reservation.path)
						.with_context(|| format!("could not create hardlink ({} -> {})", res.path().display(), reservation.path.display()))?;
				}
				if self.continue_with == ContinueWith::Link && self.enabled {
					Ok(Some(reservation.path))
				} else {
					Ok(Some(res.path().to_path_buf()))
				}
			}
			None => Ok(None),
		}
	}
}
