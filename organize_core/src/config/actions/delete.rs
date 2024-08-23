use std::path::{Path, PathBuf};

use crate::resource::Resource;
use anyhow::{Context, Result};
use serde::Deserialize;

use super::{script::ActionConfig, AsAction};

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
pub struct Delete;

impl AsAction for Delete {
	const CONFIG: ActionConfig = ActionConfig {
		requires_dest: false,
		parallelize: true,
	};

	#[tracing::instrument(ret(level = "info"), err(Debug), level = "debug", skip(_dest))]
	fn execute<T: AsRef<Path>>(&self, src: &Resource, _dest: Option<T>, dry_run: bool) -> Result<Option<PathBuf>> {
		if !dry_run {
			if src.path.is_file() {
				std::fs::remove_file(&src.path).with_context(|| format!("could not delete {}", &src.path.display()))?;
			}

			if src.path.is_dir() {
				std::fs::remove_dir_all(&src.path).with_context(|| format!("could not delete {}", &src.path.display()))?;
			}
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
		let resource = Resource::new(&tmp_file, tmp_dir.path(), vec![]);
		let action = Delete;

		std::fs::write(&tmp_file, "").expect("Could not create target file");
		assert!(tmp_file.exists());

		action
			.execute::<&Path>(&resource, None, false)
			.expect("Could not delete target file");
		assert!(!tmp_file.exists());
	}
}
