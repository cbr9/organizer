use std::path::PathBuf;

use crate::{
	config::{actions::common::enabled, context::ExecutionContext},
	errors::{ActionError, ErrorContext},
};
use anyhow::Result;
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

	#[tracing::instrument(name = "move", ret(level = "info"), err, level = "debug", skip(self, ctx, ), fields(if_exists = %self.on_conflict, path = %res.path().display()))]
	fn execute(&self, res: &Resource, ctx: &ExecutionContext) -> Result<Option<PathBuf>, ActionError> {
		let Some(target) = prepare_target_path(&self.on_conflict, res, &self.to, true, ctx)? else {
			return Ok(None);
		};

		if !ctx.settings.dry_run && self.enabled {
			std::fs::rename(res.path(), &target).map_err(|e| ActionError::Io {
				source: e,
				path: res.path().to_path_buf(),
				target: Some(target.clone().to_path_buf()),
				context: ErrorContext::from_scope(&ctx.scope),
			})?;
		}
		Ok(Some(target.to_path_buf()))
	}
}
