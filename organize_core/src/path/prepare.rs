use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::{config::actions::common::ConflictOption, string::ExpandPlaceholder};

pub fn prepare_target_path(on_conflict: &ConflictOption, src: &Path, dest: &Path) -> Result<Option<PathBuf>> {
	// if there are any placeholders in the destination, expand them
	let mut to = match dest.to_string_lossy().expand_placeholders(src) {
		Ok(str) => PathBuf::from(str),
		Err(e) => {
			log::error!("{:?}", e);
			return Err(e);
		}
	};

	if to.extension().is_none() || to.is_dir() || to.to_string_lossy().ends_with('/') {
		let filename = src.file_name();
		if filename.is_none() {
			return Ok(None);
		}
		to.push(filename.unwrap());
	}

	match dest.exists() {
		true => Ok(on_conflict.resolve_naming_conflict(&to)),
		false => Ok(Some(to)),
	}
}
