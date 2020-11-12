use crate::config::{Match, Options, UserConfig};
use serde::{Deserialize, Serialize};
use std::{fs, ops::Deref, path::PathBuf};
use toml::de::Error as TomlError;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Settings {
	#[serde(flatten)]
	pub defaults: Options,
}

impl AsRef<Self> for Settings {
	fn as_ref(&self) -> &Settings {
		self
	}
}

impl Default for Settings {
	fn default() -> Self {
		let path = UserConfig::default_dir().join("settings.toml");
		Self::new(path).unwrap()
	}
}

impl Settings {
	pub fn new(path: PathBuf) -> Result<Self, TomlError> {
		match fs::read_to_string(&path) {
			Ok(content) => {
				let settings = toml::from_str::<Settings>(&content);
				match settings {
					Ok(mut settings) => {
						settings.defaults = Options::default() + settings.defaults;
						let serialized = toml::to_string(&settings).unwrap();
						fs::write(&path, serialized).ok();
						Ok(settings)
					}
					Err(e) => Err(e),
				}
			}
			Err(_) => {
				let settings = Settings {
					defaults: Options::default(),
				};
				let serialized = toml::to_string(&settings).unwrap();
				fs::write(&path, serialized).ok();
				Ok(settings)
			}
		}
	}
}
