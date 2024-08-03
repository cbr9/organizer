use std::path::{Path, PathBuf};

use crate::config::actions::ActionType;
use anyhow::{Context, Result};
use dialoguer::{theme::ColorfulTheme, Confirm};
use serde::Deserialize;

use super::ActionPipeline;

fn enabled() -> bool {
	true
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Delete {
	#[serde(default = "enabled")]
	pub confirm: bool,
}

impl ActionPipeline for Delete {
	const REQUIRES_DEST: bool = false;
	const TYPE: ActionType = ActionType::Delete;

	fn execute<T: AsRef<Path> + Into<PathBuf> + Clone, P: AsRef<Path> + Into<PathBuf> + Clone>(
		&self,
		src: T,
		_: Option<P>,
		simulated: bool,
	) -> Result<Option<PathBuf>> {
		if !simulated {
			std::fs::remove_file(&src).with_context(|| format!("could not delete {}", src.as_ref().display()))?;
		}
		Ok(None)
	}

	fn confirm<T: AsRef<Path> + Into<PathBuf> + Clone, P: AsRef<Path> + Into<PathBuf> + Clone>(&self, src: T, _: Option<P>) -> Result<bool> {
		if self.confirm {
			Confirm::with_theme(&ColorfulTheme::default())
				.with_prompt(format!("Permanently delete {}?", src.as_ref().display()))
				.interact()
				.context("Could not interact")
		} else {
			Ok(true)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use tempfile;

	#[test]
	fn test_delete() {
		let tmp_dir = tempfile::tempdir().expect("Couldn't create temporary directory");
		let tmp_path = tmp_dir.path().to_owned();
		let tmp_file = tmp_path.join("delete_me.txt");
		let action = Delete { confirm: false };

		std::fs::write(&tmp_file, "").expect("Could create target file");
		assert!(tmp_file.exists());

		action
			.execute::<&Path, &Path>(&tmp_file, None, false)
			.expect("Could not delete target file");
		assert!(!tmp_file.exists());
	}
}
