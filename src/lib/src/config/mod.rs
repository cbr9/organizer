pub use actions::*;
pub use filters::*;
pub use folders::*;
pub use options::*;

mod actions;
mod filters;
mod folders;
mod options;

use std::{
	borrow::Cow,
	collections::{hash_map::RandomState, HashMap, HashSet},
	env,
	fs,
	ops::{Deref, DerefMut},
	path::{Path, PathBuf},
};

use anyhow::Result;
use dirs::{config_dir, home_dir};
use log::error;
use notify::RecursiveMode;
use serde::Deserialize;

use crate::{path::Update, settings::Settings, PROJECT_NAME};

// TODO: add tests for the custom deserializers

/// Represents the user's configuration file
/// ### Fields
/// * `path`: the path the user's config, either the default one or some other passed with the --with-config argument
/// * `rules`: a list of parsed rules defined by the user
#[derive(Deserialize, Clone, Debug)]
pub struct UserConfig {
	pub rules: Rules,
	pub defaults: Option<Options>,
}

impl AsRef<Self> for UserConfig {
	fn as_ref(&self) -> &UserConfig {
		self
	}
}

impl UserConfig {
	/// Creates a new UserConfig instance.
	/// It parses the configuration file
	/// and fills missing fields with either the defaults, in the case of global options,
	/// or with the global options, in the case of folder-level options.
	/// If the config file does not exist, it is created.
	/// ### Errors
	/// This constructor fails in the following cases:
	/// - The configuration file does not exist
	pub fn new<T>(path: T) -> Result<Self>
	where
		T: AsRef<Path>,
	{
		let path = path.as_ref();
		if path == UserConfig::default_path() {
			match home_dir() {
				None => panic!("error: cannot determine home directory"),
				Some(home) => env::set_current_dir(&home).unwrap(),
			};
		} else if let Some(parent) = path.parent() {
			match env::set_current_dir(parent) {
				Ok(_) => {}
				Err(e) => {
					error!("{}", e);
					std::process::exit(1);
				}
			}
		};

		if !path.exists() {
			Self::create(&path);
		}

		let content = fs::read_to_string(&path).unwrap(); // if there is some problem with the config file, we should not try to fix it
		match serde_yaml::from_str::<UserConfig>(&content) {
			Ok(mut config) => {
				let settings = Settings::new().unwrap();
				config.defaults = Some(settings.defaults).combine(config.defaults);
				for rule in config.rules.iter_mut() {
					rule.options = config.defaults.clone().combine(rule.options.clone());
					for folder in rule.folders.iter_mut() {
						folder.options = rule.options.clone().combine(folder.options.clone());
					}
					rule.options = None;
				}
				config.defaults = None;
				Ok(config)
			}
			Err(e) => {
				error!("{}", e);
				Err(e.into())
			}
		}
	}

	pub fn create(path: &Path) {
		let path = if path.exists() {
			path.update(&ConflictOption::Rename, &Default::default()).unwrap() // safe unwrap (can only return an error if if_exists == Skip)
		} else {
			Cow::Borrowed(path)
		};

		match path.parent() {
			Some(parent) => {
				if !parent.exists() {
					std::fs::create_dir_all(parent).unwrap_or_else(|_| panic!("error: could not create config directory ({})", parent.display()));
				}
				let output = include_str!("../../../../examples/config.yml");
				std::fs::write(&path, output).unwrap_or_else(|_| panic!("error: could not create config file ({})", path.display()));
				println!("New config file created at {}", path.display());
			}
			None => panic!("config file's parent folder should be defined"),
		}
	}

	pub fn default_dir() -> PathBuf {
		Self::default_path().parent().unwrap().to_path_buf()
	}

	pub fn default_path() -> PathBuf {
		config_dir().unwrap().join(PROJECT_NAME).join("config.yml")
	}
}

pub trait AsMap<'a, V> {
	fn map(&'a self) -> HashMap<&'a Path, V>;
}

#[derive(Deserialize, Clone, Debug)]
pub struct Rules(Vec<Rule>);

impl Deref for Rules {
	type Target = Vec<Rule>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl DerefMut for Rules {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl<'a> AsMap<'a, Vec<(&'a Rule, usize)>> for Rules {
	fn map(&'a self) -> HashMap<&'a Path, Vec<(&'a Rule, usize)>, RandomState> {
		let mut map = HashMap::new();
		for rule in self.iter() {
			for (i, folder) in rule.folders.iter().enumerate() {
				if !map.contains_key(folder.path.as_path()) {
					map.insert(folder.path.as_path(), Vec::new());
				}
				map.get_mut(folder.path.as_path()).unwrap().push((rule, i));
			}
		}
		map
	}
}

impl<'a> AsMap<'a, RecursiveMode> for Rules {
	fn map(&'a self) -> HashMap<&'a Path, RecursiveMode, RandomState> {
		let mut folders = HashMap::new();
		for rule in self.iter() {
			for folder in rule.folders.iter() {
				let recursive = if folder.options.as_ref().unwrap().recursive.unwrap() {
					RecursiveMode::Recursive
				} else {
					RecursiveMode::NonRecursive
				};
				let path = folder.path.as_path();
				match folders.get(path) {
					None => {
						folders.insert(path, recursive);
					}
					Some(value) => {
						if recursive == RecursiveMode::Recursive && value == &RecursiveMode::NonRecursive {
							folders.insert(path, recursive);
						}
					}
				}
			}
		}
		folders
	}
}

impl Rules {
	pub fn get_paths(&self) -> HashSet<&Path> {
		let mut set = HashSet::new();
		for rule in self.iter() {
			for folder in rule.folders.iter() {
				set.insert(folder.path.as_path());
			}
		}
		set
	}
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
	pub actions: Actions,
	pub filters: Filters,
	pub folders: Folders,
	pub options: Option<Options>,
}

impl AsRef<Self> for Rule {
	fn as_ref(&self) -> &Rule {
		self
	}
}

pub type PathToRules<'a> = HashMap<&'a Path, Vec<(&'a Rule, usize)>, RandomState>;
pub type PathToRecursive<'a> = HashMap<&'a Path, RecursiveMode, RandomState>;
