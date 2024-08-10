use std::path::{Path, PathBuf};

use anyhow::{Context as ErrorContext, Result};
use serde::Deserialize;

use crate::{path::prepare_target_path, resource::Resource, templates::Template};

use super::{common::ConflictOption, script::ActionConfig, AsAction};

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Hardlink {
	to: Template,
	#[serde(default)]
	if_exists: ConflictOption,
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
	const CONFIG: ActionConfig = ActionConfig {
		requires_dest: true,
		parallelize: true,
	};

	fn get_target_path(&self, src: &Resource) -> Result<Option<PathBuf>> {
		prepare_target_path(&self.if_exists, src, &self.to, true)
	}

	#[tracing::instrument(ret(level = "info"), err, level = "debug", skip(dest))]
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
