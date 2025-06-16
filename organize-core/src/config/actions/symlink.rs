use std::path::{Path, PathBuf};

use anyhow::{Context as ErrorContext, Result};
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
pub struct Symlink {
	to: Template,
	#[serde(default)]
	if_exists: ConflictOption,
	#[serde(default)]
	confirm: bool,
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

#[typetag::serde(name = "symlink")]
impl Action for Symlink {
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
					Self::atomic(res.path(), &dest).with_context(|| "Failed to symlink file")?;
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
