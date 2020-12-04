pub mod actions;
pub mod filters;
pub mod folders;

use std::{
	borrow::Cow,
	fs,
	path::{Path, PathBuf},
};

use anyhow::Context;
use anyhow::Result;
use dirs::{config_dir, home_dir};
use log::error;

use serde::Deserialize;

use crate::{
	data::{
		config::{
			actions::{io_action::ConflictOption, Actions},
			filters::Filters,
			folders::Folders,
		},
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
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Config {
	pub rules: Vec<Rule>,
	#[serde(default = "Options::default_none")]
	pub defaults: Options,
}

impl AsRef<Self> for Config {
	fn as_ref(&self) -> &Config {
		self
	}
}

impl Config {
	/// Creates a new UserConfig instance.
	/// It parses the configuration file
	/// and fills missing fields with either the defaults, in the case of global options,
	/// or with the global options, in the case of folder-level options.
	/// If the config file does not exist, it is created.
	/// ### Errors
	/// This constructor fails in the following cases:
	/// - The configuration file does not exist
	pub fn parse<T>(path: T) -> Result<Config>
	where
		T: AsRef<Path>,
	{
		let config_path = path.as_ref();
		let parse = |path: &Path| {
			if !path.exists() {
				Self::create(&path)?;
			}
			fs::read_to_string(&path).map(|content| serde_yaml::from_str(&content).map_err(anyhow::Error::new))?
		};

		if config_path == Self::default_path() {
			home_dir()
				.map(|path| std::env::set_current_dir(&path).map(|_| parse(&config_path))?)
				.ok_or_else(|| anyhow::Error::msg("could not determine home directory"))?
		} else {
			config_path
				.parent()
				.map(|path| std::env::set_current_dir(path).map(|_| parse(config_path))?)
				.ok_or_else(|| anyhow::Error::msg("cannot determine config file directory"))?
		}
	}

	pub fn create(path: &Path) -> anyhow::Result<()> {
		let path = if path.exists() {
			path.update(&ConflictOption::Rename, &Default::default()).unwrap() // safe unwrap (can only return an error if if_exists == Skip)
		} else {
			Cow::Borrowed(path)
		};

		path.parent()
			.map(|parent| {
				if !parent.exists() {
					std::fs::create_dir_all(parent).unwrap_or_else(|_| panic!("error: could not create config directory ({})", parent.display()));
				}
				let output = include_str!("../../../../examples/config.yml");
				std::fs::write(&path, output).unwrap_or_else(|_| panic!("error: could not create config file ({})", path.display()));
				println!("New config file created at {}", path.display());
			})
			.ok_or_else(|| anyhow::Error::msg("config file's parent folder should be defined"))
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
			|e| {
				// if the current dir could not be identified
				error!("{}", e);
				Self::default_path()
			},
			|dir| {
				dir.read_dir().map_or_else(
					|e| {
						// if it could be identified but there was a problem reading its content
						error!("{}", e);
						Self::default_path()
					},
					|mut files| {
						// if its content was successfully read, look for a `organize.yml` file, otherwise return the default
						files
							.find_map(|file| {
								if let Ok(entry) = file {
									let path = entry.path();
									let mime_type = mime_guess::from_path(&entry.path()).first_or_octet_stream();
									if path.file_stem().unwrap_or_default() == "organize" && mime_type == "text/x-yaml" {
										Some(path)
									} else {
										None
									}
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

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Rule {
	pub actions: Actions,
	pub filters: Filters,
	pub folders: Folders,
	#[serde(default = "Options::default_none")]
	pub options: Options,
}

impl Default for Rule {
	fn default() -> Self {
		Self {
			actions: Actions(vec![]),
			filters: Filters { inner: vec![] },
			folders: vec![],
			options: Options::default_none(),
		}
	}
}

impl<'a> AsRef<Self> for Rule {
	fn as_ref(&self) -> &Rule {
		self
	}
}
