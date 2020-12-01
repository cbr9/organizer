use crate::{
	config::{options::Options, UserConfig},
	settings::Settings,
	utils::{DefaultOpt, UnwrapRef},
};
use log::error;
use notify::RecursiveMode;
use serde_yaml::Error;
use std::{
	collections::{
		hash_map::{Iter, Keys},
		HashMap,
	},
	path::{Path, PathBuf},
};

pub struct Data<'a> {
	pub(crate) defaults: Options,
	pub(crate) settings: Settings,
	pub(crate) config: UserConfig,
	pub path_to_rules: PathToRules<'a>,
	pub path_to_recursive: PathToRecursive<'a>,
}

pub struct PathToRules<'a>(HashMap<&'a PathBuf, Vec<(usize, usize)>>);

impl<'a> PathToRules<'a> {
	pub fn new(config: &'a UserConfig) -> Self {
		let mut map = HashMap::with_capacity(config.rules.len());
		config.rules.iter().enumerate().for_each(|(i, rule)| {
			rule.folders.iter().enumerate().for_each(|(j, folder)| {
				let path = &folder.path;
				if !map.contains_key(path) {
					map.insert(path, Vec::new());
				}
				map.get_mut(path).unwrap().push((i, j));
			})
		});
		map.shrink_to_fit();
		Self(map)
	}

	pub fn keys(&self) -> Keys<'_, &'a PathBuf, Vec<(usize, usize)>> {
		self.0.keys()
	}

	pub fn get(&self, key: &PathBuf) -> &Vec<(usize, usize)> {
		self.0.get(key).unwrap_or_else(|| {
			// if the path is some subdirectory not represented in the hashmap
			let components = key.components().collect::<Vec<_>>();
			let mut paths = Vec::new();
			for i in 0..components.len() {
				let slice = components[0..i].iter().map(|comp| comp.as_os_str().to_string_lossy()).collect::<Vec<_>>();
				let str: String = slice.join(&std::path::MAIN_SEPARATOR.to_string());
				paths.push(PathBuf::from(str.replace("//", "/")))
			}
			let path = paths.iter().rev().find_map(|path| self.0.contains_key(path).then_some(path)).unwrap();
			self.0.get(path).unwrap()
		})
	}
}

pub struct PathToRecursive<'a>(HashMap<&'a Path, RecursiveMode>);

impl<'a> PathToRecursive<'a> {
	pub fn new(defaults: &'a Options, settings: &'a Settings, config: &'a UserConfig) -> Self {
		let mut map = HashMap::with_capacity(config.rules.len());
		config.rules.iter().for_each(|rule| {
			rule.folders.iter().for_each(|folder| {
				let recursive = folder.options.recursive.as_ref().unwrap_or_else(|| {
					rule.options.recursive.as_ref().unwrap_or_else(|| {
						config
							.defaults
							.recursive
							.as_ref()
							.unwrap_or_else(|| settings.defaults.recursive.as_ref().unwrap_or_else(|| defaults.recursive.unwrap_ref()))
					})
				});
				let recursive = if *recursive {
					RecursiveMode::Recursive
				} else {
					RecursiveMode::NonRecursive
				};
				match map.get(folder.path.as_path()) {
					None => {
						map.insert(folder.path.as_path(), recursive);
					}
					Some(value) => {
						if recursive == RecursiveMode::Recursive && value == &RecursiveMode::NonRecursive {
							map.insert(folder.path.as_path(), recursive);
						}
					}
				}
			})
		});
		map.shrink_to_fit();
		Self(map)
	}

	pub fn keys(&self) -> Keys<'_, &'a Path, RecursiveMode> {
		self.0.keys()
	}

	pub fn iter(&self) -> Iter<'_, &'a Path, RecursiveMode> {
		self.0.iter()
	}

	pub fn get(&self, key: &Path) -> Option<&RecursiveMode> {
		self.0.get(key)
	}

	pub fn insert(&mut self, key: &'a Path, value: RecursiveMode) -> Option<RecursiveMode> {
		self.0.insert(key, value)
	}
}

impl<'a> Data<'a> {
	pub fn new() -> Self {
		match UserConfig::new(UserConfig::path()) {
			Ok(config) => {
				let defaults = Options::default_some();
				let settings = Settings::from_default_path();
				let path_to_recursive = PathToRecursive::new(&defaults, &settings, &config);
				let path_to_rules = PathToRules::new(&config);
				Self {
					defaults,
					settings,
					config,
					path_to_recursive,
					path_to_rules,
				}
			}
			Err(e) => {
				error!("{}", e);
				std::process::exit(0)
			}
		}
	}
}

impl<'a> From<UserConfig> for Data<'a> {
	fn from(config: UserConfig) -> Self {
		let defaults = Options::default_some();
		let settings = Settings::from_default_path();
		let path_to_recursive = PathToRecursive::new(&defaults, &settings, &config);
		let path_to_rules = PathToRules::new(&config);
		Self {
			defaults,
			settings,
			config,
			path_to_recursive,
			path_to_rules,
		}
	}
}
