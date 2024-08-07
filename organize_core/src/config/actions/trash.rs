use std::path::{Path, PathBuf};

use crate::{config::actions::ActionType, resource::Resource, PROJECT_NAME};
use anyhow::{Context, Result};
use serde::Deserialize;

use super::AsAction;

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

impl AsAction for Trash {
	const REQUIRES_DEST: bool = false;
	const TYPE: ActionType = ActionType::Trash;

	fn execute<T: AsRef<Path>>(&self, src: &Resource, _: Option<T>, dry_run: bool) -> Result<Option<PathBuf>> {
		if !dry_run {
			let to = Self::dir()?.join(src.path.file_name().unwrap());
			let from = &src.path;
			std::fs::copy(from, &to).with_context(|| format!("Could not copy file ({} -> {})", from.display(), to.display()))?;
			std::fs::remove_file(from).with_context(|| format!("could not move ({} -> {})", from.display(), to.display()))?;
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
		let tmp_file = tempfile::NamedTempFile::new().unwrap();
		let path = tmp_file.path();
		let resource = Resource::new(path, path.parent().unwrap(), &[]);
		let action = Trash { confirm: false };

		assert!(path.exists());

		action
			.execute::<&Path>(&resource, None, false)
			.expect("Could not trash target file");
		assert!(!path.exists());
	}
}
