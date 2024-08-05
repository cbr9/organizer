use std::path::PathBuf;

use anyhow::Result;
use path_clean::PathClean;
use serde::Deserialize;
use tera::Context;

use crate::{config::options::FolderOptions, path::Expand, templates::TERA};

#[derive(Deserialize, Debug, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct Folder {
	path: PathBuf,
	#[serde(flatten, default = "FolderOptions::default_none")]
	pub options: FolderOptions,
	#[serde(default)]
	pub interactive: bool,
}

impl Folder {
	pub fn path(&self) -> Result<PathBuf> {
		let context = Context::new();
		let path = TERA
			.lock()
			.unwrap()
			.render_str(&self.path.to_string_lossy(), &context)
			.map(PathBuf::from)
			.map(|p| p.expand_user().clean())?;
		Ok(path)
	}
}

pub type Folders = Vec<Folder>;
