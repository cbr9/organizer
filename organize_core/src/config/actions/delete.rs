use std::path::{Path, PathBuf};

use crate::{
	config::{actions::ActionType, SIMULATION},
	resource::Resource,
};
use anyhow::{Context, Result};
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

	fn execute<T: AsRef<Path>>(&self, src: &Resource, _: Option<T>) -> Result<Option<PathBuf>> {
		if !*SIMULATION {
			std::fs::remove_file(src.path().as_ref()).with_context(|| format!("could not delete {}", src.path().as_ref().display()))?;
		}
		Ok(None)
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
		let mut resource = Resource::new(&tmp_file, &[]);
		let action = Delete { confirm: false };

		std::fs::write(&tmp_file, "").expect("Could not create target file");
		assert!(tmp_file.exists());

		action
			.execute::<&Path>(&mut resource, None)
			.expect("Could not delete target file");
		assert!(!tmp_file.exists());
	}
}
