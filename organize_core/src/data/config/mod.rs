use std::{
	fs,
	path::{Path, PathBuf},
};

use anyhow::{ensure, anyhow, Context, Result};
use dirs::home_dir;
use serde::Deserialize;

use crate::{
	data::{
		config::{
			actions::{Actions},
			filters::Filters,
			folders::Folders,
		},
		options::Options,
	},
	utils::DefaultOpt,
	PROJECT_NAME,
};

pub mod actions;
pub mod filters;
pub mod folders;

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Config {
	pub rules: Vec<Rule>,
	#[serde(default = "Options::default_none")]
	pub defaults: Options,
}

impl Config {
	pub fn parse<T: AsRef<Path>>(path: T) -> Result<Config> {
		fs::read_to_string(&path)
			.map(|ref content| serde_yaml::from_str(content).with_context(|| format!("could not deserialize {}", path.as_ref().display())))?
	}

	pub fn set_cwd<T: AsRef<Path>>(path: T) -> Result<PathBuf> {
		if path.as_ref() == Self::default_path()? {
			home_dir()
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

	pub fn create_in<T: AsRef<Path>>(folder: T) -> Result<PathBuf> {
		let dir = folder.as_ref();
		let path = dir.join(format!("{}.yml", PROJECT_NAME));
		ensure!(!path.exists(), format!("a config file already exists in `{}`", dir.display()));
		if !dir.exists() {
			std::fs::create_dir_all(dir).with_context(|| format!("error: could not create config directory ({})", dir.display()))?;
		}
		let output = include_str!("../../../../examples/blueprint.yml");
		std::fs::write(&path, output).with_context(|| format!("error: could not create config file ({})", path.display()))?;
		Ok(path)
	}

	pub fn default_path() -> Result<PathBuf> {
		Ok(Self::default_dir()?.join("config.yml"))
	}

	pub fn default_dir() -> Result<PathBuf> {
		let var = "ORGANIZE_CONFIG_DIR";
		std::env::var_os(var).map_or_else(
			|| {
				Ok(dirs::config_dir()
					.ok_or_else(|| anyhow!("could not find config directory, please set {} manually", var))?
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
				let path = file.ok()?.path();
				let extension = path.extension().unwrap_or_default();
				if path.file_stem().unwrap_or_default() == PROJECT_NAME && (extension == "yaml" || extension == "yml") {
					Some(path)
				} else {
					None
				}
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
			filters: Filters(vec![]),
			folders: vec![],
			options: Options::default_none(),
		}
	}
}

#[cfg(test)]
mod tests {
	use anyhow::{anyhow, Result};

	use crate::utils::tests::{project, AndWait};

	use super::*;
	use std::fs::File;

	#[test]
	fn create() -> Result<()> {
		let dir = std::env::current_dir()?;
		let path = Config::create_in(&dir);
		let first = path.is_ok();
		let second = Config::create_in(&dir).is_err();
		File::remove_and_wait(path?)?;
		assert!(first);
		assert!(second);
		Ok(())
	}

	#[test]
	fn path_custom_yml() -> Result<()> {
		let config: PathBuf = format!("{}.yml", PROJECT_NAME).into();
		File::create_and_wait(&config)?;
		let is_ok = config.canonicalize()? == Config::path()?;
		File::remove_and_wait(config)?;
		assert!(is_ok);
		let config: PathBuf = format!("{}.yaml", PROJECT_NAME).into();
		File::create_and_wait(&config)?;
		let is_ok = config.canonicalize()? == Config::path()?;
		File::remove_and_wait(config)?;
		assert!(is_ok);
		Ok(())
	}

	#[test]
	fn path_custom_default() -> Result<()> {
		["yml", "yaml"].iter().for_each(|extension| {
			let path = format!("{}.{}", PROJECT_NAME, extension);
			assert!(PathBuf::from(path).canonicalize().is_err()) // assert they don't exist in the current dir
		});
		assert_eq!(Config::default_path()?, Config::path()?);
		Ok(())
	}

	#[test]
	fn set_cwd_default() -> Result<()> {
		let cwd = Config::set_cwd(Config::default_path()?)?;
		assert_eq!(cwd, home_dir().ok_or_else(|| anyhow!("cannot determine home directory"))?);
		Ok(())
	}

	#[test]
	fn set_cwd_custom() -> Result<()> {
		let project_root = project();
		std::env::set_current_dir(&project_root)?;
		let cwd = Config::set_cwd("examples/config.yml")?;
		assert_eq!(cwd, Path::new("examples/"));
		Ok(())
	}
}
