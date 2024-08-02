use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use dialoguer::{theme::ColorfulTheme, Confirm};
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
	const REQUIRES_DEST: bool = true;
	const TYPE: ActionType = ActionType::Move;

	fn get_target_path<T: AsRef<Path> + Into<PathBuf> + Clone>(&self, src: T) -> Result<Option<PathBuf>> {
		prepare_target_path(&self.on_conflict, src.as_ref(), self.to.as_path())
	}

	fn confirm<T: AsRef<Path> + Into<PathBuf> + Clone, P: AsRef<Path> + Into<PathBuf> + Clone>(&self, src: T, dest: Option<P>) -> Result<bool> {
		if self.confirm {
			Confirm::with_theme(&ColorfulTheme::default())
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
		simulated: bool,
	) -> Result<Option<PathBuf>> {
		let dest: PathBuf = dest.unwrap().into();

		if !simulated {
			std::fs::rename(src.clone(), dest.clone())
				.with_context(|| format!("Could not move {} -> {}", src.clone().as_ref().display(), dest.display()))?;
		}

		Ok(Some(dest))
	}
}
