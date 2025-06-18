use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
	config::{actions::common::enabled, context::ExecutionContext},
	errors::{ActionError, ErrorContext},
	path::prepare::prepare_target_path,
	resource::Resource,
	templates::template::Template,
};

use super::{common::ConflictResolution, Action};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Hardlink {
	to: Template,
	#[serde(default, rename = "if_exists")]
	on_conflict: ConflictResolution,
	#[serde(default)]
	continue_with: ContinueWith,
	#[serde(default = "enabled")]
	enabled: bool,
}

#[derive(Deserialize, Default, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum ContinueWith {
	Original,
	#[default]
	Link,
}

#[typetag::serde(name = "hardlink")]
impl Action for Hardlink {
	fn templates(&self) -> Vec<&Template> {
		vec![&self.to]
	}

	#[tracing::instrument(ret(level = "info"), err(Debug), level = "debug", skip(ctx))]
	fn execute(&self, res: &Resource, ctx: &ExecutionContext) -> Result<Option<PathBuf>, ActionError> {
		match prepare_target_path(&self.on_conflict, res, &self.to, true, ctx)? {
			Some(target) => {
				if !ctx.settings.dry_run && self.enabled {
					std::fs::hard_link(res.path(), &target).map_err(|e| ActionError::Io {
						source: e,
						path: res.path().to_path_buf(),
						target: Some(target.clone().to_path_buf()),
						context: ErrorContext::from_scope(&ctx.scope),
					})?;
				}
				if self.continue_with == ContinueWith::Link && self.enabled {
					Ok(Some(target.to_path_buf()))
				} else {
					Ok(Some(res.path().to_path_buf()))
				}
			}
			None => Ok(None),
		}
	}
}
