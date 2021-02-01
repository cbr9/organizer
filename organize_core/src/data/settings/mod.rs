use std::{
	fs,
	path::{Path, PathBuf},
};

use serde::Serialize;

use crate::{
	data::{config::Config, options::Options},
	utils::DefaultOpt,
};

use std::io::ErrorKind;

use anyhow::Result;
mod de;

#[derive(Serialize, Eq, PartialEq, Debug, Clone)]
pub struct Settings {
	#[serde(flatten)]
	pub defaults: Options,
}

impl DefaultOpt for Settings {
	fn default_none() -> Self {
		Self {
			defaults: DefaultOpt::default_none(),
		}
	}

	fn default_some() -> Self {
		Self {
			defaults: DefaultOpt::default_some(),
		}
	}
}

impl From<Options> for Settings {
	fn from(opts: Options) -> Self {
		Self { defaults: opts }
	}
}

impl Settings {
	pub fn new<T: AsRef<Path>>(path: T) -> Result<Self> {
		let path = path.as_ref();
		match fs::read_to_string(&path) {
			Ok(content) => toml::from_str(&content).map_err(Into::into),
			Err(e) if e.kind() == ErrorKind::NotFound => Ok(Settings::default_some()),
			Err(e) => Err(e.into()),
		}
	}

	pub fn path() -> Result<PathBuf> {
		Config::default_dir().map(|dir| dir.join("settings.toml"))
	}
}
