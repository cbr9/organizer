use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
	config::{
		actions::{common::enabled, Change, Output},
		context::ExecutionContext,
	},
	errors::{Error, ErrorContext},
	path::prepare::prepare_target_path,
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

	fn execute(&self, ctx: &ExecutionContext) -> Result<Output, Error> {
		let Some(target) = prepare_target_path(&self.on_conflict, &self.to, true, ctx)? else {
			return Ok(Output::Continue);
		};

		if !ctx.settings.dry_run && self.enabled {
			std::fs::hard_link(ctx.scope.resource.path(), &target).map_err(|e| Error::Io {
				source: e,
				path: ctx.scope.resource.path().to_path_buf(),
				target: Some(target.clone().to_path_buf()),
				context: ErrorContext::from_scope(&ctx.scope),
			})?;
		}

		let target = target.clone().to_path_buf();
		let current = if self.continue_with == ContinueWith::Original {
			ctx.scope.resource.path().to_path_buf()
		} else {
			target.to_path_buf()
		};

		Ok(Output::Modified(Change {
			before: ctx.scope.resource.path().to_path_buf(),
			after: target,
			current,
		}))
	}
}
