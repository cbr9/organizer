use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use dialoguer::Confirm;
use serde::Deserialize;

use crate::path::prepare_target_path;

use super::{common::ConflictOption, ActionPipeline, ActionType};

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Hardlink {
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

impl ActionPipeline for Hardlink {
	const TYPE: ActionType = ActionType::Hardlink;
	const REQUIRES_DEST: bool = true;

	fn get_target_path<T: AsRef<Path> + Into<PathBuf> + Clone>(&self, src: T) -> Result<Option<PathBuf>> {
		prepare_target_path(&self.on_conflict, src.as_ref(), self.to.as_path())
	}

	fn confirm<T: AsRef<Path> + Into<PathBuf> + Clone, P: AsRef<Path> + Into<PathBuf> + Clone>(&self, src: T, dest: Option<P>) -> Result<bool> {
		if self.confirm {
			Confirm::new()
				.with_prompt(format!(
					"Hardlink {} to {}?",
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
		if !simulated {
			std::fs::hard_link(src.as_ref(), dest.as_ref().expect("dest should not be None")).with_context(|| {
				format!(
					"could not create hardlink ({} -> {})",
					src.as_ref().display(),
					dest.clone().expect("dest should not be none").as_ref().display()
				)
			})?;
		}
		if self.continue_with == ContinueWith::Link {
			Ok(Some(dest.unwrap().into()))
		} else {
			Ok(Some(src.into()))
		}
	}
}
