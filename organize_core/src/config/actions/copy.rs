use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use dialoguer::Confirm;
use serde::Deserialize;

use crate::path::prepare_target_path;

use super::{common::ConflictOption, ActionPipeline, ActionType};

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Copy {
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
	fn get_target_path<T: AsRef<Path> + Into<PathBuf> + Clone>(&self, src: T) -> Result<Option<PathBuf>> {
		prepare_target_path(&self.on_conflict, src.as_ref(), self.to.as_path())
	}

	fn confirm<T: AsRef<Path> + Into<PathBuf> + Clone, P: AsRef<Path> + Into<PathBuf> + Clone>(&self, src: T, dest: Option<P>) -> Result<bool> {
		if self.confirm {
			Confirm::new()
				.with_prompt(format!(
					"Copy {} to {}?",
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
	) -> Result<Option<PathBuf>> {
		std::fs::copy(src.as_ref(), dest.clone().unwrap().into())
			.with_context(|| "Failed to copy file")
			.map_or(Ok(None), |_| {
				if self.continue_with == ContinueWith::Copy {
					Ok(Some(dest.unwrap().into()))
				} else {
					Ok(Some(src.into()))
				}
			})
	}
}
