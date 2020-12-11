pub mod config;
pub mod options;
pub mod path_to_recursive;
pub mod path_to_rules;
pub mod settings;

use crate::{
	data::options::apply::Apply,
	data::options::r#match::Match,
	data::{config::Config, options::Options, settings::Settings},
	utils::DefaultOpt,
	utils::UnwrapRef,
	PROJECT_NAME,
};
use dirs::config_dir;

use crate::path::IsHidden;
use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Data {
	pub(crate) defaults: Options,
	pub settings: Settings,
	pub config: Config,
}

macro_rules! getter {
	(from folder, $v:vis $name:ident, $field:tt, $typ:ty) => {
		impl Data {
			$v fn $name(&self, rule: usize, folder: usize) -> &$typ {
				let rule = &self.config.rules[rule];
				let folder = &rule.folders[folder];
				folder.options.$field.as_ref().unwrap_or_else(|| {
					rule.options.$field.as_ref().unwrap_or_else(|| {
						self.config.defaults.$field.as_ref().unwrap_or_else(|| {
							self.settings.defaults.$field.as_ref().unwrap_or_else(|| {
								self.defaults.$field.unwrap_ref()
							})
						})
					})
				})
			}
		}
	};
	(from config, $v:vis $name:ident, $field:tt, $typ:ty) => {
		impl Data {
			$v fn $name(&self) -> &$typ {
				self.config.defaults.$field.as_ref().unwrap_or_else(|| {
					self.settings.defaults.$field.as_ref().unwrap_or_else(|| {
						self.defaults.$field.unwrap_ref()
					})
				})
			}
		}
	};
	(from folder, struct, $v:vis $name:ident, $field:tt, $subfield:tt, $typ:ty) => {
		impl Data {
			$v fn $name(&self, rule: usize, folder: usize) -> &$typ {
				let rule = &self.config.rules[rule];
				let folder = &rule.folders[folder];
				folder.options.$field.$subfield.as_ref().unwrap_or_else(|| {
					rule.options.$field.$subfield.as_ref().unwrap_or_else(|| {
						self.config.defaults.$field.$subfield.as_ref().unwrap_or_else(|| {
							self.settings.defaults.$field.$subfield.as_ref().unwrap_or_else(|| {
								self.defaults.$field.$subfield.unwrap_ref()
							})
						})
					})
				})
			}
		}
	};
}

getter!(from folder, pub get_recursive, recursive, bool);
getter!(from folder, pub get_watch, watch, bool);
getter!(from folder, pub get_hidden_files, hidden_files, bool);
getter!(from folder, struct, pub get_apply_actions, apply, actions, Apply);
getter!(from folder, struct, pub get_apply_filters, apply, filters, Apply);
getter!(from config, pub get_match, r#match, Match);

impl Data {
	pub fn new() -> Result<Self> {
		let data = Config::parse(Config::path()).map(|config| {
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

	pub fn should_ignore<T: AsRef<Path>>(&self, path: T, rule: usize, folder: usize) -> bool {
		let path = path.as_ref();
		if let Some(vec) = &self.settings.defaults.ignored_dirs {
			if vec
				.iter()
				.any(|ignored_path| path.parent().map(|parent| ignored_path == parent).unwrap_or_default())
			{
				return true;
			}
		}
		if let Some(vec) = &self.config.defaults.ignored_dirs {
			if vec
				.iter()
				.any(|ignored_path| path.parent().map(|parent| ignored_path == parent).unwrap_or_default())
			{
				return true;
			}
		}
		if let Some(vec) = &self.config.rules[rule].options.ignored_dirs {
			if vec
				.iter()
				.any(|ignored_path| path.parent().map(|parent| ignored_path == parent).unwrap_or_default())
			{
				return true;
			}
		}
		if let Some(vec) = &self.config.rules[rule].folders[folder].options.ignored_dirs {
			if vec
				.iter()
				.any(|ignored_path| path.parent().map(|parent| ignored_path == parent).unwrap_or_default())
			{
				return true;
			}
		}
		if path.is_hidden() && !*self.get_hidden_files(rule, folder) {
			return true;
		}
		false
	}
}
