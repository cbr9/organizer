use crate::config::{Options, UserConfig};
use anyhow::Result;
use log::{debug, error};
use serde::{Deserialize, Deserializer, Serialize};
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

impl<'de> Deserialize<'de> for Settings {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		Ok(Self::from(Options::default() + Options::deserialize(deserializer)?))
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
		fs::read_to_string(path).map_or_else(
			|e| {
				// if there is some problem with the settings file
				debug!("{}", e.to_string());
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
	use serde_test::{assert_de_tokens, Token};
	use std::path::Path;

	#[test]
	fn deserialize() {
		let mut defaults = Options::default();
		defaults.watch = Some(false);
		defaults.hidden_files = Some(true);
		defaults.recursive = Some(true);
		let value = Settings { defaults };
		assert_de_tokens(&value, &[
			Token::Map { len: Some(3) },
			Token::Str("hidden_files"),
			Token::Some,
			Token::Bool(true),
			Token::Str("watch"),
			Token::Some,
			Token::Bool(false),
			Token::Str("recursive"),
			Token::Some,
			Token::Bool(true),
			Token::MapEnd,
		])
	}
	#[test]
	fn non_existent() {
		let path = Path::new("non_existent.toml");
		let settings = Settings::new(path);
		let exists = path.exists();
		std::fs::remove_file(path).unwrap();
		assert!(exists && settings == Settings::default())
	}
}
