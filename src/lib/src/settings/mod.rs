use crate::config::{Apply, ApplyWrapper, Options, UserConfig};
use serde::{Deserialize, Serialize};
use std::{fs, ops::Add, path::PathBuf};
use toml::de::Error as TomlError;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Settings {
	#[serde(skip)]
	path: PathBuf,
	pub defaults: Options,
	// pub r#match: Match,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all(deserialize = "lowercase"))]
pub enum Match {
	All,
	First,
}

impl Default for Match {
	fn default() -> Self {
		Self::First
	}
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
			defaults: Default::default(), // r#match: Default::default()
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
						settings.defaults = Options::default() + settings.defaults;
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
