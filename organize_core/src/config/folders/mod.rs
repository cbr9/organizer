use std::path::PathBuf;

use anyhow::{Context as ErrorContext, Result};
use path_clean::PathClean;
use serde::Deserialize;
use tera::Context;

use crate::{config::options::Options, path::expand::Expand, templates::Template};

#[derive(Deserialize, Debug, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct Folder {
	path: Template,
	#[serde(flatten, default = "Options::default_none")]
	pub options: Options,
	#[serde(default)]
	pub interactive: bool,
}

impl Folder {
	pub fn path(&self) -> Result<PathBuf> {
		let context = Context::new();
		let path = self
			.path
			.render(&context)
			.with_context(|| "cannot expand folder name")
			.map(PathBuf::from)
			.and_then(|p| p.canonicalize().map_err(anyhow::Error::msg))
			.map(|p| p.expand_user().clean())?;
		Ok(path)
	}
}

pub type Folders = Vec<Folder>;
