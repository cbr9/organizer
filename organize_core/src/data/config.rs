use std::{
	fs,
	path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;

use crate::{
	data::{actions::Actions, filters::Filters, folders::Folders, options::Options},
	utils::DefaultOpt,
	PROJECT_NAME,
};

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Config {
	pub rules: Vec<Rule>,
	#[serde(default = "Options::default_none")]
	pub defaults: Options,
}

impl Config {
	pub fn create_in<T: AsRef<Path>>(folder: T) -> Result<PathBuf> {
		let path = folder.as_ref().join(format!("{}.yml", PROJECT_NAME));
		if path.exists() {
			bail!("a config file already exists in `{}`", folder.as_ref().display())
		}
		let output = include_str!("../../../examples/blueprint.yml");
		std::fs::write(&path, output).with_context(|| format!("error: could not create config file ({})", path.display()))?;
		Ok(path.canonicalize()?)
	}

	pub fn create_in_cwd() -> Result<PathBuf> {
		let dir = std::env::current_dir()?;
		Self::create_in(dir)
	}

	pub fn default_dir() -> Result<PathBuf> {
		let var = "ORGANIZE_CONFIG_DIR";
		let path = std::env::var_os(var).map_or_else(
			|| {
				Ok(dirs_next::config_dir()
					.ok_or_else(|| anyhow!("could not find config directory, please set {} manually", var))?
					.join(PROJECT_NAME))
			},
			|path| Ok(PathBuf::from(path)),
		);

		if let Ok(path) = &path {
			if !path.exists() {
				std::fs::create_dir_all(&path).expect("could not create config directory");
			}
		}
		path
	}

	pub fn default_path() -> Result<PathBuf> {
		Self::default_dir().map(|dir| dir.join("config.yml")).map(|path| {
			if !path.exists() {
				std::fs::File::create(&path).expect("Could not create config file...");
			}
			path
		})
	}

	pub fn parse<T: AsRef<Path>>(path: T) -> Result<Config> {
		fs::read_to_string(&path).map(|ref content| {
			if content.is_empty() {
				bail!("empty configuration")
			}

			toml::from_str(content).with_context(|| format!("could not deserialize {}", path.as_ref().display()))
		})?
	}

	pub fn path() -> Result<PathBuf> {
		std::env::current_dir()
			.context("cannot determine current directory")?
			.read_dir()
			.context("cannot determine directory content")?
			.find_map(|file| {
				let path = file.ok()?.path();
				let extension = path.extension().unwrap_or_default();
				let stem = path.file_stem().unwrap_or_default();
				if stem == PROJECT_NAME && (extension == "yaml" || extension == "yml") {
					Some(path)
				} else {
					None
				}
			})
			.map_or_else(Self::default_path, Ok)
	}

	pub fn set_cwd<T: AsRef<Path>>(path: T) -> Result<PathBuf> {
		if path.as_ref() == Self::default_path()? {
			dirs_next::home_dir()
				.map(|path| -> Result<PathBuf> {
					std::env::set_current_dir(&path).map_err(anyhow::Error::new)?;
					Ok(path)
				})
				.ok_or_else(|| anyhow!("could not determine home directory"))?
		} else {
			path.as_ref()
				.parent()
				.map(|path| -> Result<PathBuf> {
					std::env::set_current_dir(path).map_err(anyhow::Error::new)?;
					Ok(path.into())
				})
				.ok_or_else(|| anyhow!("could not determine config directory"))?
		}
	}
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
// #[serde(deny_unknown_fields)]
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
			filters: Filters(vec![]),
			folders: vec![],
			options: Options::default_none(),
		}
	}
}
