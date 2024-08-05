use std::path::{Path, PathBuf};

use crate::{
	config::{actions::ActionType, SIMULATION},
	resource::Resource,
	PROJECT_NAME,
};
use anyhow::{Context, Result};
use serde::Deserialize;

use super::ActionPipeline;

fn enabled() -> bool {
	true
}

#[derive(Debug, Clone, Deserialize, Default, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Trash {
	#[serde(default = "enabled")]
	pub confirm: bool,
}

impl Trash {
	fn dir() -> Result<PathBuf> {
		let dir = dirs::data_local_dir().unwrap().join(PROJECT_NAME).join(".trash");
		std::fs::create_dir_all(&dir)
			.with_context(|| format!("Could not create trash directory at {}", &dir.display()))
			.map(|_| dir)
	}
}

impl ActionPipeline for Trash {
	const REQUIRES_DEST: bool = false;
	const TYPE: ActionType = ActionType::Trash;

	fn execute<T: AsRef<Path>>(&self, src: &mut Resource, _: Option<T>) -> Result<Option<PathBuf>> {
		if !*SIMULATION {
			let to = Self::dir()?.join(src.path().as_ref().file_name().unwrap());
			let from = src.path();
			std::fs::copy(from.as_ref(), &to).with_context(|| format!("Could not copy file ({} -> {})", from.display(), to.display()))?;
			std::fs::remove_file(from.as_ref()).with_context(|| format!("could not move ({} -> {})", from.display(), to.display()))?;
		}
		Ok(None)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use tempfile;

	#[test]
	fn test_trash() {
		let tmp_dir = tempfile::tempdir().expect("Couldn't create temporary directory");
		let tmp_path = tmp_dir.path().to_owned();
		let tmp_file = tmp_path.join("trash_me.txt");
		let mut resource = Resource::new(&tmp_file, &[]);
		let action = Trash { confirm: false };

		std::fs::write(&tmp_file, "").expect("Could create target file");
		assert!(tmp_file.exists());

		action
			.execute::<&Path>(&mut resource, None)
			.expect("Could not trash target file");
		assert!(!tmp_file.exists());
	}
}
