use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use dialoguer::Confirm;
use serde::Deserialize;

use crate::path::prepare_target_path;

use super::{common::ConflictOption, ActionPipeline, ActionType};

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Move {
	pub to: PathBuf,
	#[serde(default)]
	pub on_conflict: ConflictOption,
	#[serde(default)]
	pub confirm: bool,
}

impl ActionPipeline for Move {
	const TYPE: ActionType = ActionType::Move;
	const REQUIRES_DEST: bool = true;
	fn get_target_path<T: AsRef<Path> + Into<PathBuf> + Clone>(&self, src: T) -> Result<Option<PathBuf>> {
		prepare_target_path(&self.on_conflict, src.as_ref(), self.to.as_path())
	}

	fn confirm<T: AsRef<Path> + Into<PathBuf> + Clone, P: AsRef<Path> + Into<PathBuf> + Clone>(&self, src: T, dest: Option<P>) -> Result<bool> {
		if self.confirm {
			Confirm::new()
				.with_prompt(format!(
					"Move {} to {}?",
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
		std::fs::rename(src, dest.as_ref().expect("Destination path should not be none"))
			.with_context(|| "Failed to move file")
			.map_or(Ok(None), |_| Ok(dest.map(|s| s.into())))
	}
}
