use config::{Config as LayeredConfig, File};
use itertools::Itertools;
use rule::Rule;
use std::{
	borrow::Cow,
	collections::HashSet,
	iter::FromIterator,
	path::{Path, PathBuf},
};

use anyhow::{Context as ErrorContext, Result};
use serde::Deserialize;

use crate::{utils::DefaultOpt, PROJECT_NAME};

use self::options::Options;

pub mod actions;
pub mod filters;
pub mod folders;
pub mod options;
pub mod rule;
pub mod variables;

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
	pub rules: Vec<Rule>,
	#[serde(skip)]
	pub path: PathBuf,
	#[serde(default = "Options::default_none")]
	pub defaults: Options,
}

impl Config {
	pub fn default_dir() -> PathBuf {
		let var = format!("{}_CONFIG", PROJECT_NAME.to_uppercase());
		std::env::var_os(&var).map_or_else(
			|| {
				dirs::config_dir()
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
			dirs::home_dir().context("could not determine home directory").and_then(|path| {
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

	pub fn filter_rules(&self, tags: Option<&Vec<String>>, ids: Option<&Vec<String>>) -> Vec<&Rule> {
		if let Some(tags) = tags {
			return self.filter_rules_by_tag(tags);
		}

		if let Some(ids) = ids {
			return self.filter_rules_by_id(ids);
		}

		self.rules.iter().collect_vec()
	}

	pub fn filter_rules_by_tag(&self, tags: &Vec<String>) -> Vec<&Rule> {
		let chosen_tags: HashSet<&String> = HashSet::from_iter(tags);
		let all_tags = self.rules.iter().flat_map(|r| &r.tags).collect_vec();

		for tag in tags {
			if !all_tags.contains(&tag) && !all_tags.contains(&&tag.replacen('!', "", 1)) {
				println!("no tag named {}", tag);
				return vec![];
			}
		}

		self.rules
			.iter()
			.filter(|rule| {
				chosen_tags.iter().any(|tag| {
					let mut tag = Cow::Borrowed(*tag);
					let mut negate = false;
					if tag.starts_with('!') {
						tag = Cow::Owned(tag.into_owned().replacen('!', "", 1));
						negate = true;
					}

					if *tag == "always" {
						return !negate;
					}

					if *tag == "never" {
						return negate;
					}

					let mut matches = rule.tags.contains(&*tag);
					if negate {
						matches = !matches;
					}
					matches
				})
			})
			.collect_vec()
	}

	pub fn filter_rules_by_id(&self, ids: &Vec<String>) -> Vec<&Rule> {
		let chosen_ids: HashSet<&String> = HashSet::from_iter(ids);
		let all_ids = self.rules.iter().flat_map(|r| &r.id).collect_vec();

		for id in ids {
			if !all_ids.contains(&id) && !all_ids.contains(&&id.replacen('!', "", 1)) {
				println!("no tag named {}", id);
				return vec![];
			}
		}

		self.rules
			.iter()
			.filter(|rule| {
				chosen_ids.iter().any(|id| {
					let mut id = Cow::Borrowed(*id);
					let mut negate = false;
					dbg!(&id);
					if id.starts_with('!') {
						id = Cow::Owned(id.to_mut().replacen('!', "", 1));
						negate = true;
					}

					let mut matches = rule.id.as_ref().is_some_and(|rule_id| *rule_id == *id);
					if negate {
						matches = !matches;
					}
					matches
				})
			})
			.collect_vec()
	}
}
