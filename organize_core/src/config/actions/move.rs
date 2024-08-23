use std::path::{Path, PathBuf};

use anyhow::{Context as ErrorContext, Result};
use serde::Deserialize;

use crate::{path::prepare::prepare_target_path, resource::Resource, templates::Template};

use super::{common::ConflictOption, script::ActionConfig, AsAction};

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Move {
	pub to: Template,
	#[serde(default)]
	pub if_exists: ConflictOption,
}

impl AsAction for Move {
	const CONFIG: ActionConfig = ActionConfig {
		requires_dest: true,
		parallelize: true,
	};

	fn get_target_path(&self, src: &Resource) -> Result<Option<PathBuf>> {
		prepare_target_path(&self.if_exists, src, &self.to, true)
	}

	#[tracing::instrument(ret(level = "info"), err(Debug), level = "debug", skip(dest))]
	fn execute<T: AsRef<Path>>(&self, src: &Resource, dest: Option<T>, dry_run: bool) -> Result<Option<PathBuf>> {
		let dest = dest.unwrap();
		if !dry_run {
			std::fs::rename(&src.path, dest.as_ref())
				.with_context(|| format!("Could not move {} -> {}", src.path.display(), dest.as_ref().display()))?;
		}

		Ok(Some(dest.as_ref().to_path_buf()))
	}
}
