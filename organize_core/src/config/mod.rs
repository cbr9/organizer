use config::{Config as LayeredConfig, File};
use rule::Rule;
use std::path::{Path, PathBuf};

use anyhow::{Context as ErrorContext, Result};
use serde::Deserialize;

use crate::{utils::DefaultOpt, PROJECT_NAME};

use self::options::FolderOptions;

pub mod actions;
pub mod filters;
pub mod folders;
pub mod options;
pub mod rule;

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Config {
	pub rules: Vec<Rule>,
	#[serde(skip)]
	pub path: PathBuf,
	#[serde(rename = "defaults", default = "FolderOptions::default_none")]
	pub defaults: FolderOptions,
}

pub struct Context {
	pub current_rule: usize,
	pub current_folder: usize,
}

impl Config {
	pub fn default_dir() -> PathBuf {
		let var = "ORGANIZE_CONFIG";
		std::env::var_os(var).map_or_else(
			|| {
				dirs_next::config_dir()
					.unwrap_or_else(|| panic!("could not find config directory, please set {} manually", var))
					.join(PROJECT_NAME)
			},
			PathBuf::from,
		)
	}

	pub fn default_path() -> PathBuf {
		Self::default_dir().join("config.toml")
	}

	pub fn new<T: AsRef<Path>>(path: T) -> Result<Self> {
		let mut config: Config = LayeredConfig::builder()
			.add_source(File::from(path.as_ref()))
			.build()?
			.try_deserialize::<Self>()
			.context("Could not deserialize config")?;
		config.path = path.as_ref().to_path_buf();
		Ok(config)
	}

	pub fn path() -> Result<PathBuf> {
		std::env::current_dir()
			.context("Cannot determine current directory")?
			.read_dir()
			.context("Cannot determine directory content")?
			.find_map(|file| {
				let path = file.ok()?.path();
				if path.is_dir() && path.file_stem()?.to_string_lossy().ends_with(PROJECT_NAME) {
					return Some(path.join("config.toml"));
				} else if path.file_stem()?.to_string_lossy().ends_with(PROJECT_NAME) && path.extension()? == "toml" {
					return Some(path);
				}
				None
			})
			.map_or_else(
				|| Ok(Self::default_path()),
				|path| path.canonicalize().context("Couldn't find config file"),
			)
	}

	pub fn set_cwd<T: AsRef<Path>>(path: T) -> Result<PathBuf> {
		let path = path.as_ref();
		if path == Self::default_path() {
			dirs_next::home_dir()
				.context("could not determine home directory")
				.and_then(|path| {
					std::env::set_current_dir(&path).context("Could not change into home directory")?;
					Ok(path)
				})
		} else {
			path.parent()
				.context("could not determine parent directory")
				.and_then(|path| {
					std::env::set_current_dir(path)?;
					Ok(path.to_path_buf())
				})
				.context("could not determine config directory")
		}
	}
}
