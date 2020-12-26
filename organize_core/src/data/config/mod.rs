use std::{
	fs,
	path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use dirs::home_dir;
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

pub mod actions;
pub mod filters;
pub mod folders;

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
	pub fn parse<T: AsRef<Path>>(path: T) -> Result<Config> {
		fs::read_to_string(&path).map(|content| serde_yaml::from_str(&content).map_err(anyhow::Error::new))?
	}

	pub fn set_cwd<T: AsRef<Path>>(path: T) -> Result<PathBuf> {
		if path.as_ref() == Self::default_path()? {
			home_dir()
				.map(|path| -> Result<PathBuf> {
					std::env::set_current_dir(&path).map_err(anyhow::Error::new)?;
					Ok(path)
				})
				.ok_or_else(|| anyhow::Error::msg("could not determine home directory"))?
		} else {
			path.as_ref()
				.parent()
				.map(|path| -> Result<PathBuf> {
					std::env::set_current_dir(path).map_err(anyhow::Error::new)?;
					Ok(path.into())
				})
				.ok_or_else(|| anyhow::Error::msg("could not determine config directory"))?
		}
	}

	pub fn create<T: AsRef<Path>>(path: T) -> anyhow::Result<()> {
		let path = if path.as_ref().exists() {
			path.as_ref().update(&ConflictOption::Rename, &Default::default()).unwrap() // safe unwrap (can only return an error if if_exists == Skip)
		} else {
			path.as_ref().into()
		};

		path.parent()
			.map(|parent| {
				if !parent.exists() {
					std::fs::create_dir_all(parent).unwrap_or_else(|_| panic!("error: could not create config directory ({})", parent.display()));
				}
				let output = include_str!("../../../../examples/blueprint.yml");
				std::fs::write(&path, output).unwrap_or_else(|_| panic!("error: could not create config file ({})", path.display()));
				println!("New config file created at {}", path.display());
			})
			.ok_or_else(|| anyhow::Error::msg("config file's parent folder should be defined"))
	}

	pub fn default_path() -> Result<PathBuf> {
		Ok(Self::default_dir()?.join("config.yml"))
	}

	pub fn default_dir() -> Result<PathBuf> {
		let var = "ORGANIZE_CONFIG_DIR";
		std::env::var_os(var).map_or_else(
			|| {
				Ok(dirs::config_dir()
					.ok_or_else(|| anyhow::Error::msg(format!("could not find config directory, please set {var} manually", var = var)))?
					.join(PROJECT_NAME))
			},
			|path| Ok(PathBuf::from(path)),
		)
	}

	pub fn path() -> Result<PathBuf> {
		std::env::current_dir()
			.context("cannot determine current directory")?
			.read_dir()
			.context("could not determine directory content")?
			.find_map(|file| {
				file.ok().map(|entry| {
					let path = entry.path();
					let mime_type = mime_guess::from_path(&entry.path()).first_or_octet_stream();
					if path.file_stem().unwrap_or_default() == "organize" && mime_type == "text/x-yaml" {
						Some(path)
					} else {
						None
					}
				})?
			})
			.map_or_else(Self::default_path, Ok)
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

#[cfg(test)]
mod tests {
	use anyhow::Result;

	use crate::utils::tests::project;

	use super::*;

	#[test]
	fn set_cwd() -> Result<()> {
		let project_root = project();
		if std::env::current_dir()? != project_root {
			std::env::set_current_dir(&project_root)?;
		}
		Config::set_cwd(Config::default_path()?).map(|cwd| -> Result<()> {
			std::env::set_current_dir(&project_root)?;
			assert_eq!(cwd, home_dir().ok_or_else(|| anyhow::Error::msg("cannot determine home directory"))?);
			Ok(())
		})??;
		Config::set_cwd("examples/config.yml").map(|cwd| -> Result<()> {
			std::env::set_current_dir(project_root)?;
			assert_eq!(cwd, Path::new("examples/"));
			Ok(())
		})??;
		Ok(())
	}
}
