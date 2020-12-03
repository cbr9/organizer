mod de;

use crate::{
	data::{config::UserConfig, options::Options},
	utils::DefaultOpt,
};

use log::{debug};
use serde::Serialize;
use std::{
	fs,
	path::{Path, PathBuf},
};

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
	pub fn new<T>(path: T) -> Result<Self, toml::de::Error>
	where
		T: AsRef<Path>,
	{
		let path = path.as_ref();
		fs::read_to_string(path).map_or_else(
			|e| {
				// if there is some problem with the settings file
				debug!("{:?}", e);
				let settings = Settings::default_some();
				// Serialize is automatically derived and these are default options so it's safe to unwrap
				let serialized = toml::to_string(&settings).unwrap();
				fs::write(path, serialized).unwrap_or_else(|e| debug!("{}", e));
				Ok(settings)
			},
			|content| {
				// if the file exists and could be read
				toml::from_str::<Settings>(&content)
			},
		)
	}

	pub fn path() -> PathBuf {
		UserConfig::default_dir().join("settings.toml")
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::Path;

	#[test]
	fn non_existent() {
		let path = Path::new("non_existent.toml");
		let settings = Settings::new(path).unwrap();
		let exists = path.exists(); // Settings::new should create a new settings file if the given path does not exist
		std::fs::remove_file(path).unwrap();
		assert!(exists && settings == Settings::default_some())
	}
}
