use std::path::{Path, PathBuf};

use crate::resource::Resource;
use anyhow::{Context, Result};
use serde::Deserialize;

use super::{script::ActionConfig, AsAction};

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
pub struct Delete;

impl<'a> AsAction<'a> for Delete {
	const CONFIG: ActionConfig<'a> = ActionConfig {
		requires_dest: false,
		log_hint: "DELETE",
	};

	fn execute<T: AsRef<Path>>(&self, src: &Resource, _: Option<T>, dry_run: bool) -> Result<Option<PathBuf>> {
		if !dry_run {
			std::fs::remove_file(&src.path).with_context(|| format!("could not delete {}", &src.path.display()))?;
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
		let resource = Resource::new(&tmp_file, tmp_dir.path(), &[]);
		let action = Delete;

		std::fs::write(&tmp_file, "").expect("Could not create target file");
		assert!(tmp_file.exists());

		action
			.execute::<&Path>(&resource, None, false)
			.expect("Could not delete target file");
		assert!(!tmp_file.exists());
	}
}
