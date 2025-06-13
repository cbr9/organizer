use std::path::PathBuf;

use anyhow::{Context as ErrorContext, Result};
use serde::{Deserialize, Serialize};

use crate::{path::prepare::prepare_target_path, resource::Resource, templates::Template};

use super::{common::ConflictOption, script::ActionConfig, Action};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Move {
	pub to: Template,
	#[serde(default)]
	pub if_exists: ConflictOption,
}

#[typetag::serde(name = "move")]
impl Action for Move {
	fn config(&self) -> ActionConfig {
		ActionConfig {
			requires_dest: true,
			parallelize: true,
		}
	}

	fn get_target_path(&self, src: &Resource) -> Result<Option<PathBuf>> {
		prepare_target_path(&self.if_exists, src, &self.to, true)
	}

	#[tracing::instrument(ret(level = "info"), err(Debug), level = "debug", skip(dest))]
	fn execute(&self, src: &Resource, dest: Option<PathBuf>, dry_run: bool) -> Result<Option<PathBuf>> {
		let dest = dest.unwrap();
		if !dry_run {
			std::fs::rename(&src.path, &dest).with_context(|| format!("Could not move {} -> {}", src.path.display(), dest.display()))?;
		}

		Ok(Some(dest))
	}
}
