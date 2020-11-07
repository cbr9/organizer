use crate::config::{Apply, ApplyWrapper, Options, UserConfig};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use toml::de::Error as TomlError;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Settings {
	#[serde(skip)]
	path: PathBuf,
	pub defaults: Options,
}

impl AsRef<Self> for Settings {
	fn as_ref(&self) -> &Settings {
		self
	}
}

impl Default for Settings {
	fn default() -> Self {
		Self {
			path: PathBuf::new(),
			defaults: Options {
				ignore: Some(Vec::new()),
				hidden_files: Some(false),
				recursive: Some(false),
				watch: Some(true),
				apply: Some(ApplyWrapper::from(Apply::All)),
			},
		}
	}
}

impl Settings {
	pub fn new() -> Result<Self, TomlError> {
		let path = UserConfig::dir().join("settings.toml");
		match fs::read_to_string(&path) {
			Ok(content) => {
				let settings = toml::from_str::<Settings>(&content);
				match settings {
					Ok(mut settings) => {
						let defaults = Settings::default();
						settings.defaults = &defaults.defaults + &settings.defaults;
						Ok(settings)
					}
					Err(e) => Err(e),
				}
			}
			Err(_) => {
				let default = Settings::default();
				let serialized = toml::to_string(&default).unwrap();
				fs::write(&path, serialized).ok();
				Ok(default)
			}
		}
	}
}
