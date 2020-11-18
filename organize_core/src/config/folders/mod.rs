mod de;

use std::{path::PathBuf, str::FromStr};

use crate::{config::options::Options, path::Expand};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Folder {
	pub path: PathBuf,
	pub options: Option<Options>,
}

impl FromStr for Folder {
	type Err = std::io::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let path = PathBuf::from(s);
		match path.expand_user().expand_vars().canonicalize() {
			Ok(path) => Ok(Self { path, options: None }),
			Err(e) => Err(e),
		}
	}
}

pub type Folders = Vec<Folder>;
