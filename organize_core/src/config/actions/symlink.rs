use std::path::{Path, PathBuf};

use anyhow::{Context as ErrorContext, Result};
use serde::Deserialize;

use crate::{config::SIMULATION, path::prepare_target_path, resource::Resource};

use super::{common::ConflictOption, ActionPipeline, ActionType};

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Symlink {
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

impl ActionPipeline for Symlink {
	const REQUIRES_DEST: bool = true;
	const TYPE: ActionType = ActionType::Symlink;

	fn get_target_path(&self, src: &Resource) -> Result<Option<PathBuf>> {
		prepare_target_path(&self.if_exists, src, self.to.as_path(), true)
	}

	fn execute<T: AsRef<Path>>(&self, src: &Resource, dest: Option<T>) -> Result<Option<PathBuf>> {
		let dest = dest.unwrap();
		if !*SIMULATION {
			Self::atomic(src.path().as_ref(), &dest).with_context(|| "Failed to symlink file")?;
		}
		if self.continue_with == ContinueWith::Link {
			Ok(Some(dest.as_ref().to_path_buf()))
		} else {
			Ok(Some(src.path().into_owned()))
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
