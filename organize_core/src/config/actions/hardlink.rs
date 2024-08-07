use std::path::{Path, PathBuf};

use anyhow::{Context as ErrorContext, Result};
use serde::Deserialize;

use crate::{path::prepare_target_path, resource::Resource};

use super::{common::ConflictOption, AsAction, ActionType};

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Hardlink {
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
	Link,
}

impl Default for ContinueWith {
	fn default() -> Self {
		Self::Original
	}
}

impl AsAction for Hardlink {
	const REQUIRES_DEST: bool = true;
	const TYPE: ActionType = ActionType::Hardlink;

	fn get_target_path(&self, src: &Resource) -> Result<Option<PathBuf>> {
		prepare_target_path(&self.if_exists, src, self.to.as_path(), true)
	}

	fn execute<T: AsRef<Path>>(&self, src: &Resource, dest: Option<T>, dry_run: bool) -> Result<Option<PathBuf>> {
		let dest = dest.unwrap();
		if !dry_run {
			std::fs::hard_link(&src.path, dest.as_ref())
				.with_context(|| format!("could not create hardlink ({} -> {})", src.path.display(), dest.as_ref().display()))?;
		}
		if self.continue_with == ContinueWith::Link {
			Ok(Some(dest.as_ref().to_path_buf()))
		} else {
			Ok(Some(src.path.clone()))
		}
	}
}
