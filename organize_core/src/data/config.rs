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
	pub fn parse<T: AsRef<Path>>(path: T) -> Result<Config> {
		fs::read_to_string(&path)
			.map(|ref content| {
				if content.is_empty() {
					bail!("empty configuration")
				}
				serde_yaml::from_str(content).with_context(|| format!("could not deserialize {}", path.as_ref().display()))
			})?
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

	pub fn create_in_cwd() -> Result<PathBuf> {
		let dir = std::env::current_dir()?;
		Self::create_in(dir)
	}

	pub fn create_in<T: AsRef<Path>>(folder: T) -> Result<PathBuf> {
		let path = folder.as_ref().join(format!("{}.yml", PROJECT_NAME));
		if path.exists() {
			bail!("a config file already exists in `{}`", folder.as_ref().display())
		}
		let output = include_str!("../../../examples/blueprint.yml");
		std::fs::write(&path, output).with_context(|| format!("error: could not create config file ({})", path.display()))?;
		Ok(path.canonicalize()?)
	}

	pub fn default_path() -> Result<PathBuf> {
		Self::default_dir().map(|dir| dir.join("config.yml"))
	}

	pub fn default_dir() -> Result<PathBuf> {
		let var = "ALFRED_CONFIG_DIR";
		std::env::var_os(var).map_or_else(
			|| {
				Ok(dirs_next::config_dir()
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
	use std::fs::File;

	use anyhow::{anyhow, Result};

	use crate::utils::tests::{project, AndWait};

	use super::*;

	#[test]
	fn test() -> Result<()> {
		// these tests are all crammed into one to avoid concurrency issues
		// where one test interferes with the setup on another
		create()?;
		path_custom()?;
		path_default()?;
		set_cwd_custom()?;
		set_cwd_default()
	}

	fn create() -> Result<()> {
		let path = Config::create_in_cwd();
		let first = path.is_ok();
		let second = Config::create_in_cwd().is_err();
		File::remove_and_wait(path?)?;
		assert!(first);
		assert!(second);
		Ok(())
	}

	fn path_custom() -> Result<()> {
		let cwd = std::env::current_dir()?;
		if cwd == dirs_next::home_dir().unwrap_or_default() {}
		let config: PathBuf = format!("{}.yml", PROJECT_NAME).into();
		File::create_and_wait(&config)?;
		let is_ok = config.canonicalize()? == Config::path()?;
		File::remove_and_wait(config)?;
		assert!(is_ok);
		// let config: PathBuf = format!("{}.yaml", PROJECT_NAME).into();
		// File::create_and_wait(&config)?;
		// let is_ok = config.canonicalize()? == Config::path()?;
		// File::remove_and_wait(config)?;
		// assert!(is_ok);
		Ok(())
	}

	fn path_default() -> Result<()> {
		["yml", "yaml"].iter().for_each(|extension| {
			let path = format!("{}.{}", PROJECT_NAME, extension);
			assert!(PathBuf::from(path).canonicalize().is_err()) // assert they don't exist in the current dir
		});
		assert_eq!(Config::default_path()?, Config::path()?);
		Ok(())
	}

	fn set_cwd_default() -> Result<()> {
		let cwd = Config::set_cwd(Config::default_path()?)?;
		assert_eq!(cwd, dirs_next::home_dir().ok_or_else(|| anyhow!("cannot determine home directory"))?);
		Ok(())
	}

	fn set_cwd_custom() -> Result<()> {
		let project_root = project();
		std::env::set_current_dir(&project_root)?;
		let cwd = Config::set_cwd("examples/config.yml")?;
		assert_eq!(cwd, Path::new("examples/"));
		Ok(())
	}
}
