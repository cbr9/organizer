use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::{path::prepare_target_path, resource::Resource, templates::Template};

use super::{common::ConflictOption, script::ActionConfig, AsAction};

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Copy {
	to: Template,
	#[serde(default)]
	if_exists: ConflictOption,
	#[serde(default)]
	continue_with: ContinueWith,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
enum ContinueWith {
	Original,
	Copy,
}

impl Default for ContinueWith {
	fn default() -> Self {
		Self::Copy
	}
}

impl<'a> AsAction<'a> for Copy {
	const CONFIG: ActionConfig<'a> = ActionConfig {
		requires_dest: true,
		log_hint: "COPY",
	};

	fn get_target_path(&self, src: &Resource) -> Result<Option<PathBuf>> {
		prepare_target_path(&self.if_exists, src, &self.to, true)
	}

	fn execute<T: AsRef<Path>>(&self, src: &Resource, dest: Option<T>, dry_run: bool) -> Result<Option<PathBuf>> {
		let dest = dest.unwrap();
		if !dry_run {
			std::fs::copy(&src.path, &dest).with_context(|| "Failed to copy file")?;
		}

		if self.continue_with == ContinueWith::Copy {
			Ok(Some(dest.as_ref().to_path_buf()))
		} else {
			Ok(Some(src.path.clone()))
		}
	}
}
