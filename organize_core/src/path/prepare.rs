use path_clean::PathClean;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};

use anyhow::{bail, Result};

use crate::{config::actions::common::ConflictOption, resource::Resource, templates::TERA};

use super::Expand;

pub fn prepare_target_path(if_exists: &ConflictOption, src: &Resource, dest: &Path, with_extension: bool) -> Result<Option<PathBuf>> {
	// if there are any placeholders in the destination, expand them

	let path = &src.path;
	let mut to = match TERA.lock().unwrap().render_str(&dest.to_string_lossy(), &src.context) {
		Ok(str) => PathBuf::from(str).expand_user().clean(),
		Err(e) => {
			log::error!("{:?}", e);
			bail!("something went wrong");
		}
	};

	if to.extension().is_none() || to.is_dir() || to.to_string_lossy().ends_with(MAIN_SEPARATOR) {
		if with_extension {
			let filename = path.file_name();
			if filename.is_none() {
				return Ok(None);
			}
			to.push(filename.unwrap());
		} else {
			let stem = path.file_stem();
			if stem.is_none() {
				return Ok(None);
			}
			to.push(stem.unwrap())
		}
	}

	Ok(if_exists.resolve_naming_conflict(&to))
}
