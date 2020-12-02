pub mod actions;
pub mod filters;
pub mod folders;
pub mod options;

use std::{
	borrow::Cow,
	collections::HashMap,
	fs,
	ops::{Deref, DerefMut},
	path::{Path, PathBuf},
};

use anyhow::Context;
use dirs::{config_dir, home_dir};

use notify::RecursiveMode;
use serde::Deserialize;

use crate::{
	data::config::{
		actions::{io_action::ConflictOption, Actions},
		filters::Filters,
		folders::Folders,
		options::Options,
	},
	path::Update,
	utils::DefaultOpt,
	PROJECT_NAME,
};

// TODO: add tests for the custom deserializers

/// Represents the user's configuration file
/// ### Fields
/// * `path`: the path the user's config, either the default one or some other passed with the --with-config argument
/// * `rules`: a list of parsed rules defined by the user
#[derive(Deserialize, Clone, Debug)]
pub struct UserConfig {
	pub rules: Rules,
	#[serde(default = "Options::default_none")]
	pub defaults: Options,
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
	pub fn new<T>(path: T) -> serde_yaml::Result<UserConfig>
	where
		T: AsRef<Path>,
	{
		let path = path.as_ref();
		if path == Self::default_path() {
			std::env::set_current_dir(home_dir().unwrap()).unwrap();
		} else {
			std::env::set_current_dir(path.parent().unwrap()).unwrap();
		}
		println!("{}", path.display());

		if !path.exists() {
			Self::create(&path);
		}
		let content = fs::read_to_string(&path).unwrap(); // if there is some problem with the config file, we should not try to fix it
		serde_yaml::from_str::<UserConfig>(&content)
		// match serde_yaml::from_str::<UserConfig>(&content) {
		// 	Ok(mut config) => {
		// 		let settings = Settings::from_default_path();
		// 		config.defaults = Some(settings.defaults).combine(&config.defaults);
		// 		for rule in config.rules.iter_mut() {
		// 			rule.options = config.defaults.combine(&rule.options);
		// 			for folder in rule.folders.iter_mut() {
		// 				folder.options = rule.options.combine(&folder.options);
		// 			}
		// 			rule.options = None;
		// 		}
		// 		config.rules.path_to_rules = Some(config.rules.map());
		// 		config.rules.path_to_recursive = Some(config.rules.map());
		// 		Ok(config)
		// 	}
		// 	Err(e) => {
		// 		error!("{}", e);
		// 		Err(e.into())
		// 	}
		// }
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
		let dir = config_dir().unwrap().join(PROJECT_NAME);
		if !dir.exists() {
			std::fs::create_dir(&dir).context("could not create config directory").unwrap();
		}
		dir
	}

	pub fn default_path() -> PathBuf {
		Self::default_dir().join("config.yml")
	}

	pub fn path() -> PathBuf {
		std::env::current_dir().map_or_else(
			|_| {
				// if the current dir could not be identified
				Self::default_path()
			},
			|dir| {
				dir.read_dir().map_or_else(
					|_| {
						// if it could be identified but there was a problem reading its content
						Self::default_path()
					},
					|mut files| {
						// if its content was successfully read, look for a `organize.yml` file, otherwise return the default
						files
							.find_map(|file| {
								if let Ok(entry) = file {
									let path = entry.path();
									let mime_type = mime_guess::from_path(&entry.path()).first_or_octet_stream();
									(path.file_stem().unwrap() == "organize" && mime_type == "text/x-yaml").then_some(path)
								} else {
									None
								}
							})
							.unwrap_or_else(Self::default_path)
					},
				)
			},
		)
	}
}

#[derive(Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct Rules {
	pub(crate) inner: Vec<Rule>,
	#[serde(skip)]
	pub path_to_rules: Option<HashMap<PathBuf, Vec<(usize, usize)>>>,
	#[serde(skip)]
	pub path_to_recursive: Option<HashMap<PathBuf, RecursiveMode>>,
}

impl Deref for Rules {
	type Target = Vec<Rule>;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl DerefMut for Rules {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.inner
	}
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
	pub actions: Actions,
	pub filters: Filters,
	pub folders: Folders,
	#[serde(default = "Options::default_none")]
	pub options: Options,
}

impl<'a> AsRef<Self> for Rule {
	fn as_ref(&self) -> &Rule {
		self
	}
}
