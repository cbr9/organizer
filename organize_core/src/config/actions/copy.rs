use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::{path::prepare_target_path, resource::Resource};

use super::{common::ConflictOption, ActionPipeline, ActionType};

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Copy {
	to: PathBuf,
	#[serde(default)]
	if_exists: ConflictOption,
	#[serde(default)]
	confirm: bool,
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

impl ActionPipeline for Copy {
	const REQUIRES_DEST: bool = true;
	const TYPE: ActionType = ActionType::Copy;

	fn get_target_path(&self, src: &Resource) -> Result<Option<PathBuf>> {
		prepare_target_path(&self.if_exists, src, self.to.as_path(), true)
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
