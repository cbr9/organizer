use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use dialoguer::{theme::ColorfulTheme, Confirm};
use serde::Deserialize;

use crate::path::prepare_target_path;

use super::{common::ConflictOption, ActionPipeline, ActionType};

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Symlink {
	to: PathBuf,
	#[serde(default)]
	on_conflict: ConflictOption,
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

	fn get_target_path<T: AsRef<Path> + Into<PathBuf> + Clone>(&self, src: T) -> Result<Option<PathBuf>> {
		prepare_target_path(&self.on_conflict, src.as_ref(), self.to.as_path())
	}

	fn confirm<T: AsRef<Path> + Into<PathBuf> + Clone, P: AsRef<Path> + Into<PathBuf> + Clone>(&self, src: T, dest: Option<P>) -> Result<bool> {
		if self.confirm {
			Confirm::with_theme(&ColorfulTheme::default())
				.with_prompt(format!(
					"Symlink {} to {}?",
					src.as_ref().display(),
					dest.expect("dest should not be None").as_ref().display()
				))
				.interact()
				.context("Could not interact")
		} else {
			Ok(true)
		}
	}

	fn execute<T: AsRef<Path> + Into<PathBuf> + Clone, P: AsRef<Path> + Into<PathBuf> + Clone>(
		&self,
		src: T,
		dest: Option<P>,
		simulated: bool,
	) -> Result<Option<PathBuf>> {
		let dest = dest.unwrap();
		if !simulated {
			Self::atomic(&src, &dest).with_context(|| "Failed to symlink file")?;
		}
		if self.continue_with == ContinueWith::Link {
			Ok(Some(dest.into()))
		} else {
			Ok(Some(src.into()))
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
