use std::path::PathBuf;

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};

use crate::{
	config::{actions::common::enabled, context::Context},
	path::prepare::prepare_target_path,
	resource::Resource,
	templates::template::Template,
};

use super::{common::ConflictOption, Action};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Hardlink {
	to: Template,
	#[serde(default)]
	if_exists: ConflictOption,
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
	fn execute(&self, res: &Resource, ctx: &Context) -> Result<Option<PathBuf>> {
		match prepare_target_path(&self.if_exists, res, &self.to, true, ctx.template_engine)? {
			Some(dest) => {
				if !ctx.dry_run && self.enabled {
					if let Some(parent) = dest.parent() {
						std::fs::create_dir_all(parent).with_context(|| format!("Could not create parent directory for {}", dest.display()))?;
					}
					std::fs::hard_link(res.path(), &dest)
						.with_context(|| format!("could not create hardlink ({} -> {})", res.path().display(), dest.display()))?;
				}
				if self.continue_with == ContinueWith::Link && self.enabled {
					Ok(Some(dest))
				} else {
					Ok(Some(res.path().to_path_buf()))
				}
			}
			None => Ok(None),
		}
	}
}
