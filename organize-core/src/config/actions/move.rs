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
	#[serde(default, rename = "if_exists")]
	pub on_conflict: ConflictResolution,
	#[serde(default = "enabled")]
	enabled: bool,
}

#[typetag::serde(name = "move")]
impl Action for Move {
	fn templates(&self) -> Vec<&Template> {
		vec![&self.to]
	}

	#[tracing::instrument(name = "move", ret(level = "info"), err, level = "debug", skip(self, ctx, res), fields(if_exists = %self.on_conflict, path = %res.path().display()))]
	fn execute(&self, res: &Resource, ctx: &ExecutionContext) -> Result<Option<PathBuf>> {
		match prepare_target_path(&self.on_conflict, res, &self.to, true, ctx)? {
			Some(target) => {
				if !ctx.settings.dry_run && self.enabled {
					std::fs::rename(res.path(), &target)
						.with_context(|| format!("Could not move {} -> {}", res.path().display(), target.display()))?;
				}
				Ok(Some(target.to_path_buf()))
			}
			None => Ok(None),
		}
	}
}
