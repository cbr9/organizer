use std::{
	fs,
	path::{Path, PathBuf},
};

use log::debug;
use serde::Serialize;

use crate::{data::options::Options, utils::DefaultOpt};
use crate::data::config::Config;
use crate::data::Data;
use std::io::{Error, ErrorKind};

mod de;

#[derive(Serialize, Eq, PartialEq, Debug, Clone)]
pub struct Settings {
	#[serde(flatten)]
	pub defaults: Options,
}

impl AsRef<Self> for Settings {
	fn as_ref(&self) -> &Settings {
		self
	}
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
	pub fn new<T: AsRef<Path>>(path: T) -> anyhow::Result<Settings> {
		let path = path.as_ref();
		match fs::read_to_string(&path) {
			Ok(content) => toml::from_str(&content).map_err(anyhow::Error::new),
			Err(e) if e.kind() == ErrorKind::NotFound => Ok(Settings::default_some()),
			Err(e) => Err(e.into())
		}
	}

	pub fn path() -> anyhow::Result<PathBuf> {
		Config::default_dir().map(|dir| dir.join("settings.toml"))
	}
}
