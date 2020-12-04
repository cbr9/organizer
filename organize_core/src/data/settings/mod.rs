mod de;

use crate::{
	data::{options::Options},
	utils::DefaultOpt,
};

use log::debug;
use serde::Serialize;
use std::{
	fs,
	path::{Path, PathBuf},
};
use crate::data::Data;

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
	pub fn new<T>(path: T) -> Result<Settings, toml::de::Error>
	where
		T: AsRef<Path>,
	{
		let path = path.as_ref();
		fs::read_to_string(path).map(|str| toml::from_str(&str)).unwrap_or_else(|e| {
			debug!("{:?}", e);
			// using default_some is unnecessary as we already have a `defaults` field in crate::data::Data
			Ok(Settings::default_none())
		})
	}

	pub fn path() -> PathBuf {
		Data::dir().join("settings.toml")
	}
}

