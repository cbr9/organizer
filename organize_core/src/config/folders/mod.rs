use std::path::PathBuf;

use anyhow::Result;
use path_clean::PathClean;
use serde::Deserialize;

use crate::{
	config::options::FolderOptions,
	path::Expand,
	templates::{CONTEXT, TERA},
};

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
		let mut ctx = CONTEXT.lock().unwrap();
		let path = TERA
			.lock()
			.unwrap()
			.render_str(&self.path.to_string_lossy(), &ctx)
			.map(PathBuf::from)
			.map(|p| p.expand_user().clean())?;
		ctx.insert("root", &path.to_string_lossy());
		Ok(path)
	}
}

pub type Folders = Vec<Folder>;
