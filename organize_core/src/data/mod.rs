pub mod config;
pub mod options;
pub mod path_to_recursive;
pub mod path_to_rules;
pub mod settings;

use crate::{
	data::{config::UserConfig, options::Options, settings::Settings},
	utils::DefaultOpt,
	PROJECT_NAME,
};
use dirs::config_dir;
use log::error;

use std::path::PathBuf;

#[derive(Debug)]
pub struct Data {
	pub(crate) defaults: Options,
	pub(crate) settings: Settings,
	pub(crate) config: UserConfig,
}

impl Default for Data {
	fn default() -> Self {
		Self::new()
	}
}

impl Data {
	pub fn new() -> Self {
		match UserConfig::new(UserConfig::path()) {
			Ok(config) => Self {
				defaults: Options::default_some(),
				settings: Settings::from_default_path(),
				config,
			},
			Err(e) => {
				error!("{}", e);
				std::process::exit(0)
			}
		}
	}

	pub fn dir() -> PathBuf {
		config_dir().unwrap().join(PROJECT_NAME)
	}
}

impl From<UserConfig> for Data {
	fn from(config: UserConfig) -> Self {
		Self {
			defaults: Options::default_some(),
			settings: Settings::from_default_path(),
			config,
		}
	}
}
