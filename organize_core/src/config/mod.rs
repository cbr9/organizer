use config::{Config as LayeredConfig, File};
use itertools::Itertools;
use rule::Rule;
use std::{
	collections::HashSet,
	iter::FromIterator,
	path::{Path, PathBuf},
	sync::OnceLock,
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

pub static CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
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

	pub fn filter_rules<I, T>(&self, tags: Option<I>, ids: Option<T>) -> Vec<&Rule>
	where
		I: IntoIterator,
		T: IntoIterator,
		I::Item: AsRef<str>,
		T::Item: AsRef<str>,
	{
		if let Some(tags) = tags {
			return self.filter_rules_by_tag(tags);
		}

		if let Some(ids) = ids {
			return self.filter_rules_by_id(ids);
		}

		self.rules.iter().collect_vec()
	}

	pub fn filter_rules_by_tag<I>(&self, tags: I) -> Vec<&Rule>
	where
		I: IntoIterator,
		I::Item: AsRef<str>,
	{
		let chosen_tags: HashSet<String> = HashSet::from_iter(tags.into_iter().map(|s| s.as_ref().to_string()));
		let all_tags: HashSet<&String> = HashSet::from_iter(self.rules.iter().flat_map(|r| &r.tags));

		let positive_tags: HashSet<String> = HashSet::from_iter(chosen_tags.iter().filter(|s| !s.starts_with('!')).cloned());
		let negative_tags: HashSet<String> = HashSet::from_iter(
			chosen_tags
				.iter()
				.filter(|s| s.starts_with('!'))
				.map(|s| s.replacen('!', "", 1)),
		);

		for tag in chosen_tags.iter() {
			if !all_tags.contains(tag) && !all_tags.contains(&tag.replacen('!', "", 1)) {
				println!("no tag named {}", tag);
				return vec![];
			}
		}

		let positive: HashSet<&String> = HashSet::from_iter(
			self.rules
				.iter()
				.filter(|rule| positive_tags.iter().any(|tag| rule.tags.contains(tag)))
				.flat_map(|r| &r.tags),
		);

		let negative: HashSet<&String> = HashSet::from_iter(
			self.rules
				.iter()
				.filter(|rule| {
					if negative_tags.is_empty() {
						return false;
					}
					negative_tags.iter().all(|tag| !rule.tags.contains(tag))
				})
				.flat_map(|r| &r.tags),
		);

		let tags = positive.union(&negative).copied().collect_vec();
		self.rules
			.iter()
			.filter(|r| r.tags.iter().any(|tag| tags.contains(&tag)))
			.collect_vec()
	}

	pub fn filter_rules_by_id<I>(&self, ids: I) -> Vec<&Rule>
	where
		I: IntoIterator,
		I::Item: AsRef<str>,
	{
		let chosen_ids: HashSet<String> = HashSet::from_iter(ids.into_iter().map(|s| s.as_ref().to_string()));
		let all_ids: HashSet<String> = HashSet::from_iter(self.rules.iter().flat_map(|r| &r.id).cloned());

		let positive_ids: HashSet<String> = HashSet::from_iter(chosen_ids.iter().filter(|s| !s.starts_with('!')).cloned());
		let negative_ids: HashSet<String> = HashSet::from_iter(chosen_ids.iter().filter(|s| s.starts_with('!')).map(|s| s.replacen('!', "", 1)));

		for id in chosen_ids.iter() {
			if !all_ids.contains(id) && !all_ids.contains(&id.replacen('!', "", 1)) {
				println!("no tag named {}", id);
				return vec![];
			}
		}

		let positive: HashSet<&String> = HashSet::from_iter(
			self.rules
				.iter()
				.filter(|rule| {
					positive_ids
						.iter()
						.any(|id| rule.id.as_ref().is_some_and(|rule_id| *rule_id == *id))
				})
				.filter_map(|r| r.id.as_ref()),
		);

		let negative: HashSet<&String> = HashSet::from_iter(
			self.rules
				.iter()
				.filter(|rule| {
					if negative_ids.is_empty() {
						return false;
					}
					negative_ids
						.iter()
						.all(|id| rule.id.as_ref().is_some_and(|rule_id| *rule_id != *id))
				})
				.flat_map(|r| r.id.as_ref()),
		);

		let ids = positive.union(&negative).copied().collect_vec();
		self.rules
			.iter()
			.filter(|r| r.id.as_ref().is_some_and(|id| ids.contains(&id)))
			.collect_vec()
	}
}

#[cfg(test)]
mod tests {
	use std::sync::LazyLock;

	use super::Config;
	use itertools::Itertools;
	use toml::toml;

	static TOML: LazyLock<toml::Table> = LazyLock::new(|| {
		toml! {

				[[rules]]
				id = "test-rule-1"
				tags = ["tag1"]

				actions = []
				filters = []
				folders = []

				[[rules]]
				id = "test-rule-2"
				tags = ["tag2"]

				actions = []
				filters = []
				folders = []

				[[rules]]
				id = "test-rule-3"
				tags = ["tag3"]

				actions = []
				filters = []
				folders = []

				[[rules]]
				tags = ["tag3"]

				actions = []
				filters = []
				folders = []

				[[rules]]
				actions = []
				filters = []
				folders = []

		}
	});

	static CONFIG: LazyLock<Config> = LazyLock::new(|| toml::from_str(&TOML.to_string()).unwrap());

	#[test]
	fn filter_rules_by_tag_positive() {
		let found_rules = CONFIG.filter_rules_by_tag(["tag2"]).iter().map(|&r| r.clone()).collect_vec();
		let expected_rules = CONFIG.rules.get(1..=1).unwrap();
		assert_eq!(found_rules, expected_rules)
	}
	#[test]
	fn filter_rules_by_tag_negative() {
		let found_rules = CONFIG.filter_rules_by_tag(["!tag2"]).iter().copied().collect_vec();
		let expected_rules = vec![CONFIG.rules.first().unwrap(), CONFIG.rules.get(2).unwrap(), CONFIG.rules.get(3).unwrap()];
		assert_eq!(found_rules, expected_rules)
	}
	#[test]
	fn filter_rules_by_tag_multiple_positive() {
		let found_rules = CONFIG
			.filter_rules_by_tag(["tag2", "tag1"])
			.iter()
			.copied()
			.cloned()
			.collect_vec();
		let expected_rules = CONFIG.rules.get(..=1).unwrap();
		assert_eq!(found_rules, expected_rules)
	}
	#[test]
	fn filter_rules_by_tag_multiple_negative() {
		let found_rules = CONFIG
			.filter_rules_by_tag(["!tag2", "!tag1"])
			.iter()
			.copied()
			.cloned()
			.collect_vec();
		let expected_rules = CONFIG.rules.get(2..=3).unwrap();
		assert_eq!(found_rules, expected_rules)
	}
	#[test]
	fn filter_rules_by_tag_multiple_mixed() {
		let found_rules = CONFIG
			.filter_rules_by_tag(["tag2", "!tag1"])
			.iter()
			.copied()
			.cloned()
			.collect_vec();
		let expected_rules = CONFIG.rules.get(1..=3).unwrap();
		assert_eq!(found_rules, expected_rules)
	}
	#[test]
	fn filter_rules_by_id_positive() {
		let found_rules = CONFIG.filter_rules_by_id(["test-rule-1"]).iter().copied().collect_vec();
		let expected_rules = vec![CONFIG.rules.first().unwrap()];
		assert_eq!(found_rules, expected_rules)
	}
	#[test]
	fn filter_rules_by_id_negative() {
		let found_rules = CONFIG
			.filter_rules_by_id(["!test-rule-1"])
			.iter()
			.map(|r| (*r).clone())
			.collect_vec();
		let expected_rules = CONFIG.rules.get(1..=2).unwrap();
		assert_eq!(found_rules, expected_rules)
	}
	#[test]
	fn filter_rules_by_id_multiple_positive() {
		let found_rules = CONFIG
			.filter_rules_by_id(["test-rule-1", "test-rule-2"])
			.iter()
			.map(|&r| r.clone())
			.collect_vec();
		let expected_rules = CONFIG.rules.get(0..=1).unwrap();
		assert_eq!(found_rules, expected_rules)
	}
	#[test]
	fn filter_rules_by_id_multiple_negative() {
		let found_rules = CONFIG
			.filter_rules_by_id(["!test-rule-1", "!test-rule-2"])
			.iter()
			.copied()
			.collect_vec();
		let expected_rules = vec![CONFIG.rules.get(2).unwrap()];
		assert_eq!(found_rules, expected_rules)
	}
	#[test]
	fn filter_rules_by_id_multiple_mixed() {
		let found_rules = CONFIG
			.filter_rules_by_id(["test-rule-1", "!test-rule-2"])
			.iter()
			.copied()
			.collect_vec();
		let expected_rules = vec![CONFIG.rules.first().unwrap(), CONFIG.rules.get(2).unwrap()];
		assert_eq!(found_rules, expected_rules)
	}
}
