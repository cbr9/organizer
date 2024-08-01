use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use tera::{Context, Tera};

use crate::config::actions::common::ConflictOption;

pub fn get_context<T: AsRef<Path>>(path: T) -> Context {
	let mut context = tera::Context::new();
	let path = path.as_ref();
	context.insert("path", &path.to_string_lossy());
	context.insert("parent", &path.parent().unwrap().to_string_lossy());
	context.insert("stem", &path.file_stem().unwrap().to_string_lossy());
	context.insert("filename", &path.file_name().unwrap().to_string_lossy());
	context.insert("extension", &path.extension().unwrap().to_string_lossy());
	let hash = sha256::try_digest(&path).unwrap();
	let mime = mime_guess::from_path(&path).first_or_octet_stream().to_string();
	context.insert("hash", &hash);
	context.insert("mime", &mime);
	context
}

pub fn prepare_target_path(on_conflict: &ConflictOption, src: &Path, dest: &Path) -> Result<Option<PathBuf>> {
	// if there are any placeholders in the destination, expand them

	let context = get_context(&src);
	let mut to = match Tera::one_off(&dest.to_string_lossy(), &context, true) {
		Ok(str) => PathBuf::from(str),
		Err(e) => {
			log::error!("{:?}", e);
			bail!("something went wrong");
		}
	};

	if to.extension().is_none() || to.is_dir() || to.to_string_lossy().ends_with('/') {
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
