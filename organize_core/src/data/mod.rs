use std::path::PathBuf;

use anyhow::{anyhow, Result};

use crate::{
    data::{
        config::Config,
        options::{apply::Apply, Options, r#match::Match},
        settings::Settings,
	},
    PROJECT_NAME,
    utils::{DefaultOpt, UnwrapRef},
};

pub mod config;
pub mod options;
pub mod path_to_recursive;
pub mod path_to_rules;
pub mod settings;
pub mod actions;
pub mod filters;
pub mod folders;

#[derive(Debug, Clone)]
pub struct Data {
	pub(crate) defaults: Options,
	pub settings: Settings,
	pub config: Config,
}

macro_rules! getters {
	($($v:vis fn $name:ident(&self, rule: $rul:ty, folder: $fol:ty) -> $typ:ty {$field:tt})+) => {
		impl Data {
			$($v fn $name(&self, rule: $rul, folder: $fol) -> &$typ {
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
			})+
		}
	};
	($($v:vis fn $name:ident(&self) -> $typ:ty {$field:tt} )+) => {
		impl Data {
			$($v fn $name(&self) -> &$typ {
				self.config.defaults.$field.as_ref().unwrap_or_else(|| {
					self.settings.defaults.$field.as_ref().unwrap_or_else(|| {
						self.defaults.$field.unwrap_ref()
					})
				})
			})+
		}
	};
	($($v:vis fn $name:ident(&self, rule: $rul:ty, folder: $fol:ty) -> $typ:ty { $field:tt.$subfield:tt })+) => {
		impl Data {
			$($v fn $name(&self, rule: $rul, folder: $fol) -> &$typ {
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
			})+
		}
	};
}

getters! {
	pub fn match_rules(&self) -> Match {
		r#match
	}
}

getters! {
	pub fn allows_watching(&self, rule: usize, folder: usize) -> bool {
		watch
	}
	pub fn allows_partial_files(&self, rule: usize, folder: usize) -> bool {
		partial_files
	}
	pub fn allows_hidden_files(&self, rule: usize, folder: usize) -> bool {
		hidden_files
	}
}

getters! {
	pub fn get_recursive_depth(&self, rule: usize, folder: usize) -> u16 {
		recursive.depth
	}
	pub fn get_apply_actions(&self, rule: usize, folder: usize) -> Apply {
		apply.actions
	}
	pub fn get_apply_filters(&self, rule: usize, folder: usize) -> Apply {
		apply.filters
	}
}

impl Data {
	pub fn new() -> Result<Self> {
		let path = Config::path()?;
		let config = Config::parse(&path)?;
		Config::set_cwd(path)?;
		let settings = Settings::new(Settings::path()?)?;
		let data = Self {
			defaults: Options::default_some(),
			settings,
			config,
		};
		Ok(data)
	}

	pub fn dir() -> Result<PathBuf> {
		let var = "ALFRED_DATA_DIR";
		std::env::var_os(var).map_or_else(
			|| {
				Ok(dirs_next::data_local_dir()
					.ok_or_else(|| anyhow!("could not find data directory, please set {} manually", var))?
					.join(PROJECT_NAME))
			},
			|path| Ok(PathBuf::from(path)),
		)
	}
}
