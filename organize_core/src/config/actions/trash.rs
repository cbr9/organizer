use std::path::{Path, PathBuf};

use crate::config::actions::ActionType;
use anyhow::{Context, Result};
use dialoguer::Confirm;
use serde::Deserialize;

use super::ActionPipeline;

fn enabled() -> bool {
	true
}

#[derive(Debug, Clone, Deserialize, Default, Eq, PartialEq)]
pub struct Trash {
	#[serde(default = "enabled")]
	enable: bool,
	#[serde(default)]
	pub confirm: bool,
}

impl Trash {
	fn dir() -> Result<PathBuf> {
		let dir = dirs_next::data_local_dir().unwrap().join("organize").join(".trash");
		std::fs::create_dir_all(&dir)
			.with_context(|| format!("Could not create trash directory at {}", &dir.display()))
			.map(|_| dir)
	}
}

impl ActionPipeline for Trash {
	const TYPE: ActionType = ActionType::Trash;
	const REQUIRES_DEST: bool = false;

	fn execute<T: AsRef<Path> + Into<PathBuf> + Clone, P: AsRef<Path> + Into<PathBuf> + Clone>(
		&self,
		src: T,
		_: Option<P>,
	) -> Result<Option<PathBuf>> {
		if self.enable {
			let to = Self::dir()?.join(src.as_ref().file_name().unwrap());
			let from = src.as_ref();
			std::fs::copy(from, &to).with_context(|| format!("Could not copy file ({} -> {})", from.display(), to.display()))?;
			std::fs::remove_file(from)
				.with_context(|| format!("could not move ({} -> {})", from.display(), to.display()))
				.map(|_| None)
		} else {
			Ok(Some(src.into()))
		}
	}
	fn confirm<T: AsRef<Path> + Into<PathBuf> + Clone, P: AsRef<Path> + Into<PathBuf> + Clone>(&self, src: T, _: Option<P>) -> Result<bool> {
		if self.confirm && self.enable {
			Confirm::new()
				.with_prompt(format!("Send {} to trash?", src.as_ref().display()))
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
	fn test_trash_true() {
		let tmp_dir = tempfile::tempdir().expect("Couldn't create temporary directory");
		let tmp_path = tmp_dir.path().to_owned();
		let tmp_file = tmp_path.join("trash_me.txt");
		let action = Trash {
			enable: true,
			confirm: false,
		};

		std::fs::write(&tmp_file, "").expect("Could create target file");
		assert!(tmp_file.exists());

		action
			.execute::<&Path, &Path>(&tmp_file, None)
			.expect("Could not trash target file");
		assert!(!tmp_file.exists());
	}

	#[test]
	fn test_trash_false() {
		let tmp_dir = tempfile::tempdir().expect("Couldn't create temporary directory");
		let tmp_path = tmp_dir.path().to_owned();
		let tmp_file = tmp_path.join("trash_me.txt");
		let action = Trash {
			enable: false,
			confirm: false,
		};

		std::fs::write(&tmp_file, "").expect("Could create target file");
		assert!(tmp_file.exists());

		action
			.execute::<&Path, &Path>(&tmp_file, None)
			.expect("Could not trash target file");
		assert!(tmp_file.exists());
	}
}
