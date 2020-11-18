mod de;

use crate::config::{options::Options, UserConfig};
use anyhow::Context;
use log::{debug, error};
use serde::Serialize;
use std::{
	fs,
	path::{Path, PathBuf},
};

#[derive(Serialize, Eq, PartialEq, Debug, Clone, Default)]
pub struct Settings {
	#[serde(flatten)]
	pub defaults: Options,
}

impl AsRef<Self> for Settings {
	fn as_ref(&self) -> &Settings {
		self
	}
}

impl From<Options> for Settings {
	fn from(opts: Options) -> Self {
		Self { defaults: opts }
	}
}

impl Settings {
	pub fn new<T>(path: T) -> Self
	where
		T: AsRef<Path>,
	{
		let path = path.as_ref();
		fs::read_to_string(path).with_context(|| "problem reading settings.toml").map_or_else(
			|e| {
				// if there is some problem with the settings file
				debug!("{:?}", e);
				let settings = Settings::default();
				// Serialize is automatically derived and these are default options so it's safe to unwrap
				let serialized = toml::to_string(&settings).unwrap();
				fs::write(path, serialized).unwrap_or_else(|e| debug!("{}", e));
				settings
			},
			|content| {
				// if the file exists and could be read
				toml::from_str::<Settings>(&content).unwrap_or_else(|e| {
					error!("{}", e.to_string());
					std::process::exit(0)
				})
			},
		)
	}

	pub fn default_path() -> PathBuf {
		UserConfig::default_dir().join("settings.toml")
	}

	pub fn from_default_path() -> Self {
		Self::new(Self::default_path())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::Path;

	#[test]
	fn non_existent() {
		let path = Path::new("non_existent.toml");
		let settings = Settings::new(path);
		let exists = path.exists(); // Settings::new should create a new settings file if the given path does not exist
		std::fs::remove_file(path).unwrap();
		assert!(exists && settings == Settings::default())
	}
}
