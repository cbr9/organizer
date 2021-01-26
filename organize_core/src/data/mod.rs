use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};

use crate::path::IsHidden;
use crate::{
	data::options::apply::Apply,
	data::options::r#match::Match,
	data::{config::Config, options::Options, settings::Settings},
	utils::DefaultOpt,
	utils::UnwrapRef,
	PROJECT_NAME,
};

pub mod config;
pub mod options;
pub mod path_to_recursive;
pub mod path_to_rules;
pub mod settings;

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

getter!(from folder, struct, pub get_recursive_depth, recursive, depth, u16); // try to get recursive.depth from `folder` up until `defaults`
getter!(from folder, struct, pub get_recursive_enabled, recursive, enabled, bool);
getter!(from folder, pub get_watch, watch, bool);
getter!(from folder, pub get_hidden_files, hidden_files, bool);
getter!(from folder, struct, pub get_apply_actions, apply, actions, Apply);
getter!(from folder, struct, pub get_apply_filters, apply, filters, Apply);
getter!(from config, pub get_match, r#match, Match);

impl Data {
	pub fn new() -> Result<Self> {
		let path = Config::path()?;
		let config = Config::parse(&path)?;
		Config::set_cwd(path)?;
		let settings = Settings::new(Settings::path()?)?;
		let data = Self {
			defaults: Options::default_some(),
			settings,
			config
		};
		Ok(data)
	}

	pub fn dir() -> Result<PathBuf> {
		let var = "ORGANIZE_DATA_DIR";
		std::env::var_os(var).map_or_else(
			|| {
				Ok(dirs::data_local_dir()
					.ok_or_else(|| anyhow!("could not find data directory, please set {} manually", var))?
					.join(PROJECT_NAME))
			},
			|path| Ok(PathBuf::from(path)),
		)
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

#[cfg(test)]
mod tests {
	use crate::data::config::actions::Actions;
	use crate::data::config::filters::Filters;
	use crate::data::config::folders::Folder;
	use crate::data::config::Rule;

	use super::*;

	#[test]
	fn should_ignore() {
		let config = Path::new("$HOME/.config");
		let documents_cache = Path::new("$HOME/Documents/.cache");
		let npm = Path::new("$HOME/.npm");
		let downloads_config = Path::new("$HOME/Downloads/.config");

		let data = Data {
			defaults: Options::default_some(),
			settings: Settings {
				defaults: Options {
					ignored_dirs: Some(vec![config.into()]),
					..DefaultOpt::default_none()
				},
			},
			config: Config {
				rules: vec![Rule {
					actions: Actions(vec![]),
					filters: Filters { inner: vec![] },
					folders: vec![
						Folder {
							path: "$HOME".into(),
							options: Options {
								ignored_dirs: Some(vec![npm.into()]),
								..DefaultOpt::default_none()
							},
						},
						Folder {
							path: "$HOME/Downloads".into(),
							options: Options::default_none(),
						},
						Folder {
							path: "$HOME/Documents".into(),
							options: Options::default_none(),
						},
					],
					options: Options {
						ignored_dirs: Some(vec![documents_cache.into()]),
						..DefaultOpt::default_none()
					},
				}],
				defaults: Options {
					ignored_dirs: Some(vec![downloads_config.into()]),
					..DefaultOpt::default_none()
				},
			},
		};
		assert!(data.should_ignore(config.join("config.yml"), 0, 0));
		assert!(data.should_ignore(documents_cache.join("cache.txt"), 0, 2));
		assert!(data.should_ignore(npm.join("npm.js"), 0, 0));
		assert!(data.should_ignore(downloads_config.join("config.yml"), 0, 0));
		assert!(data.should_ignore("$HOME/.config.yml", 0, 0));
		assert!(!data.should_ignore("$HOME/config.yml", 0, 0));
	}
}
