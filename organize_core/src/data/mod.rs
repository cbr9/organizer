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

use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Data {
	pub(crate) defaults: Options,
	pub settings: Settings,
	pub config: UserConfig,
}

impl Data {
	pub fn new() -> Result<Self> {
		let data = UserConfig::parse(UserConfig::path()).map(|config| {
			Settings::new(Settings::path()).map(|settings| Self {
				defaults: Options::default_some(),
				settings,
				config,
			})
		})??; // return the error from UserConfig::parse and from Settings::new
		Ok(data)
	}

	pub fn dir() -> PathBuf {
		config_dir().unwrap().join(PROJECT_NAME)
	}
}
