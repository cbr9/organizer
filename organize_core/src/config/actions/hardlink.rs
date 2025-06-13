use std::path::PathBuf;

use anyhow::{Context as ErrorContext, Result};
use serde::{Deserialize, Serialize};

use crate::{path::prepare::prepare_target_path, resource::Resource, templates::Template};

use super::{common::ConflictOption, script::ActionConfig, Action};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Hardlink {
	to: Template,
	#[serde(default)]
	if_exists: ConflictOption,
	#[serde(default)]
	continue_with: ContinueWith,
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
			std::fs::hard_link(&src.path, &dest)
				.with_context(|| format!("could not create hardlink ({} -> {})", src.path.display(), dest.display()))?;
		}
		if self.continue_with == ContinueWith::Link {
			Ok(Some(dest))
		} else {
			Ok(Some(src.path.clone()))
		}
	}
}
