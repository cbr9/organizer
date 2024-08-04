use std::path::{Path, PathBuf};

use anyhow::{Context as ErrorContext, Result};
use dialoguer::{theme::ColorfulTheme, Confirm};
use serde::Deserialize;

use crate::path::prepare_target_path;

use super::{common::ConflictOption, ActionPipeline, ActionType};

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Hardlink {
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

impl ActionPipeline for Hardlink {
	const REQUIRES_DEST: bool = true;
	const TYPE: ActionType = ActionType::Hardlink;

	fn get_target_path<T: AsRef<Path> + Into<PathBuf> + Clone>(&self, src: T) -> Result<Option<PathBuf>> {
		prepare_target_path(&self.if_exists, src.as_ref(), self.to.as_path(), true)
	}

	fn confirm<T: AsRef<Path> + Into<PathBuf> + Clone, P: AsRef<Path> + Into<PathBuf> + Clone>(&self, src: T, dest: Option<P>) -> Result<bool> {
		if self.confirm {
			Confirm::with_theme(&ColorfulTheme::default())
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
