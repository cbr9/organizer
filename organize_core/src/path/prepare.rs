use path_clean::PathClean;
use std::{
	ops::DerefMut,
	path::{Path, PathBuf, MAIN_SEPARATOR},
};

use anyhow::{bail, Result};

use crate::{
	config::actions::common::ConflictOption,
	templates::{CONTEXT, TERA},
};

use super::Expand;

pub fn prepare_target_path(if_exists: &ConflictOption, src: &Path, dest: &Path, with_extension: bool) -> Result<Option<PathBuf>> {
	// if there are any placeholders in the destination, expand them

	let mut ctx = CONTEXT.lock().unwrap();
	let mut to = match TERA.lock().unwrap().render_str(&dest.to_string_lossy(), ctx.deref_mut()) {
		Ok(str) => PathBuf::from(str).expand_user().clean(),
		Err(e) => {
			log::error!("{:?}", e);
			bail!("something went wrong");
		}
	};

	if to.extension().is_none() || to.is_dir() || to.to_string_lossy().ends_with(MAIN_SEPARATOR) {
		let filename = src.file_name();
		if filename.is_none() {
			return Ok(None);
		}
		std::fs::create_dir_all(&to)?;
		if with_extension {
			to.push(filename.unwrap());
		} else {
			to.push(src.file_stem().unwrap())
		}
	} else {
		std::fs::create_dir_all(to.parent().unwrap())?;
	}

	match dest.exists() {
		true => Ok(if_exists.resolve_naming_conflict(&to)),
		false => Ok(Some(to)),
	}
}
