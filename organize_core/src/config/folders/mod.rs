use std::path::PathBuf;

use serde::Deserialize;

use crate::{config::options::Options, path::Expand, utils::DefaultOpt};
use std::convert::TryFrom;

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Folder {
	pub path: PathBuf,
	#[serde(default)]
	pub options: Options,
	#[serde(default)]
	pub interactive: bool,
}

impl TryFrom<PathBuf> for Folder {
	type Error = anyhow::Error;

	fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
		path.expand_user()?
			.expand_vars()?
			.canonicalize()
			.map(|path| Self {
				path,
				options: DefaultOpt::default_none(),
				interactive: false,
			})
			.map_err(anyhow::Error::new)
	}
}

pub type Folders = Vec<Folder>;
