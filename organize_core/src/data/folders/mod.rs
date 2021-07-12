mod de;

use std::{path::PathBuf, str::FromStr};

use crate::{data::options::Options, path::Expand, utils::DefaultOpt};
use std::convert::TryFrom;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Folder {
	pub path: PathBuf,
	pub options: Options,
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
			})
			.map_err(anyhow::Error::new)
	}
}

impl FromStr for Folder {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let path = PathBuf::from(s);
		Self::try_from(path)
	}
}

pub type Folders = Vec<Folder>;
