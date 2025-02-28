use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use bson::doc;
use serde::Deserialize;
use serde_json::{json, value};

use crate::{
	backup::{BACKUP_DIR, DATABASE},
	path::prepare::{prepare_target_path, PathUtils},
	resource::Resource,
	templates::Template,
};

use super::{common::ConflictOption, ActionConfig, AsAction};

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Copy {
	to: Template,
	#[serde(default)]
	if_exists: ConflictOption,
	#[serde(default)]
	continue_with: ContinueWith,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
enum ContinueWith {
	Original,
	Copy,
}

impl Default for ContinueWith {
	fn default() -> Self {
		Self::Copy
	}
}

impl AsAction for Copy {
	const CONFIG: ActionConfig = ActionConfig {
		requires_dest: true,
		parallelize: true,
	};

	fn get_target_path(&self, src: &Resource) -> Result<Option<PathBuf>> {
		prepare_target_path(&self.if_exists, src, &self.to, true)
	}

	#[tracing::instrument(ret(level = "info"), err(Debug), level = "debug", skip(dest))]
	fn execute<T: AsRef<Path>>(&self, src: &Resource, dest: Option<T>, dry_run: bool) -> Result<(Option<PathBuf>, i32)> {
		let dest = dest.unwrap().as_ref().to_path_buf();
		let mut db = DATABASE.lock().unwrap();
		let id = db.insert(
			json!({ "source": &src.path.to_string_lossy().to_string(), "destination": dest.to_string_lossy().to_string(), "dry_run": dry_run, "backup": "", "type": "copy" }),
			src.last_event_id,
		)?;

		if !dry_run {
			std::fs::copy(&src.path, &dest).with_context(|| "Failed to copy file")?;
			let mut backup_path = src.path.replace_parent(&BACKUP_DIR);
			backup_path.set_file_name(format!("{}.{}", id.to_string(), src.path.extension().unwrap().to_str().unwrap()));

			std::fs::copy(&src.path, &backup_path).with_context(|| "Failed to backup file")?;
			db.add_backup_path(id, backup_path)?;
		}

		if self.continue_with == ContinueWith::Copy {
			Ok((Some(dest), id))
		} else {
			Ok((Some(src.path.clone()), id))
		}
	}
}
