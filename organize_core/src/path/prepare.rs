use path_clean::PathClean;
use std::{
	collections::HashMap,
	ops::DerefMut,
	path::{Path, PathBuf, MAIN_SEPARATOR},
};

use anyhow::{bail, Result};
use tera::{Context};

use crate::{
	config::actions::common::ConflictOption,
	templates::{CONTEXT, TERA},
};

use super::Expand;

pub fn get_env_context() -> Context {
	let mut environment = HashMap::new();
	let mut variables = HashMap::new();
	for (key, value) in std::env::vars() {
		variables.insert(key, value);
	}
	environment.insert("env", variables);
	Context::from_serialize(environment).unwrap()
}

pub fn prepare_target_path(on_conflict: &ConflictOption, src: &Path, dest: &Path) -> Result<Option<PathBuf>> {
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
		to.push(filename.unwrap());
	} else {
		std::fs::create_dir_all(to.parent().unwrap())?;
	}

	match dest.exists() {
		true => Ok(on_conflict.resolve_naming_conflict(&to)),
		false => Ok(Some(to)),
	}
}
