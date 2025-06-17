use std::path::PathBuf;

use crate::config::{actions::common::enabled, context::ExecutionContext};
use anyhow::{Context as ErrorContext, Result};
use serde::{Deserialize, Serialize};

use crate::{path::prepare::prepare_target_path, resource::Resource, templates::template::Template};

use super::{common::ConflictResolution, Action};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Move {
	pub to: Template,
	#[serde(default)]
	pub if_exists: ConflictResolution,
	#[serde(default = "enabled")]
	enabled: bool,
}

#[typetag::serde(name = "move")]
impl Action for Move {
	fn templates(&self) -> Vec<&Template> {
		vec![&self.to]
	}

	#[tracing::instrument(ret(level = "info"), err(Debug), level = "debug", skip(ctx))]
	fn execute(&self, res: &Resource, ctx: &ExecutionContext) -> Result<Option<PathBuf>> {
		match prepare_target_path(&self.if_exists, res, &self.to, true, ctx)? {
			Some(reservation) => {
				if !ctx.settings.dry_run && self.enabled {
					std::fs::rename(res.path(), &reservation.path)
						.with_context(|| format!("Could not move {} -> {}", res.path().display(), reservation.path.display()))?;
				}
				Ok(Some(reservation.path))
			}
			None => Ok(None),
		}
	}
}
