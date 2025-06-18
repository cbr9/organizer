use std::path::{Path, PathBuf};

use anyhow::{Context as ErrorContext, Result};
use serde::{Deserialize, Serialize};

use crate::{
	config::{actions::common::enabled, context::ExecutionContext},
	path::prepare::prepare_target_path,
	resource::Resource,
	templates::template::Template,
};

use super::{common::ConflictResolution, Action};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Symlink {
	to: Template,
	#[serde(default, rename = "if_exists")]
	on_conflict: ConflictResolution,
	#[serde(default)]
	confirm: bool,
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

#[typetag::serde(name = "symlink")]
impl Action for Symlink {
	fn templates(&self) -> Vec<&Template> {
		vec![&self.to]
	}

	#[tracing::instrument(ret(level = "info"), err(Debug), level = "debug", skip(ctx))]
	fn execute(&self, res: &Resource, ctx: &ExecutionContext) -> Result<Option<PathBuf>> {
		match prepare_target_path(&self.on_conflict, res, &self.to, true, ctx)? {
			Some(target) => {
				if !ctx.settings.dry_run && self.enabled {
					Self::atomic(res.path(), &target).with_context(|| "Failed to symlink file")?;
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

impl Symlink {
	#[cfg(target_family = "unix")]
	fn atomic<T: AsRef<Path>, P: AsRef<Path>>(src: T, dest: P) -> std::io::Result<()> {
		std::os::unix::fs::symlink(src.as_ref(), dest.as_ref())
	}

	#[cfg(target_family = "windows")]
	fn atomic<T: AsRef<Path>, P: AsRef<Path>>(src: T, dest: P) -> std::io::Result<()> {
		std::os::windows::fs::symlink_file(src.as_ref(), dest.as_ref())
	}
}
