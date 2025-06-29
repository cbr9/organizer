use config::{Config as LayeredConfig, File};
use itertools::Itertools;
use std::{collections::HashSet, path::PathBuf, sync::OnceLock};
use tokio::sync::OnceCell;

use anyhow::{anyhow, Context as ErrorContext, Result};
use serde::{Deserialize, Serialize};

use crate::{options::OptionsBuilder, rule::Rule, templates::variable::Variable, PROJECT_NAME};

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Config {
	pub rules: Vec<Rule>,
	#[serde(flatten)]
	pub defaults: OptionsBuilder,
	#[serde(skip)]
	pub path: OnceCell<PathBuf>,
}

impl Config {
	pub fn new(path: Option<PathBuf>, tags: &Option<Vec<String>>, ids: &Option<Vec<String>>) -> Result<Config> {
		let path = Self::resolve_path(path);
		if !path.exists() {
			return Err(anyhow!("Configuration file not found at {}", path.display()));
		}

		let config = LayeredConfig::builder()
			.add_source(File::from(path.clone()))
			.build()?
			.try_deserialize::<Config>()?;

		config.path.set(path).unwrap();

		let mut positive_tags = HashSet::new();
		let mut negative_tags = HashSet::new();
		if let Some(tags) = tags {
			// Pre-process the tags into positive and negative sets once.
			positive_tags = tags.iter().filter(|&s| !s.starts_with('!')).cloned().collect();
			negative_tags = tags
				.iter()
				.filter(|&s| s.starts_with('!'))
				.cloned()
				.map(|s| s[1..].to_string())
				.collect();
		}

		let mut positive_ids = HashSet::new();
		let mut negative_ids = HashSet::new();
		if let Some(ids) = ids {
			// Pre-process the IDs into positive and negative sets once.
			positive_ids = ids.iter().filter(|&s| !s.starts_with('!')).cloned().collect();
			negative_ids = ids
				.iter()
				.filter(|&s| s.starts_with('!'))
				.cloned()
				.map(|s| s[1..].to_string())
				.collect();
		}

		let rules = config
			.rules
			.iter()
			.filter(|rule| rule.matches_tags(&positive_tags, &negative_tags))
			.filter(|rule| rule.matches_ids(&positive_ids, &negative_ids))
			.filter(|rule| rule.enabled)
			.cloned()
			.collect_vec();

		Ok(Config {
			rules,
			defaults: config.defaults,
			path: config.path,
		})
	}

	pub fn resolve_path(path: Option<PathBuf>) -> PathBuf {
		match path {
			Some(path) => path,
			None => {
				let organize_config_dir = format!("{}_CONFIG", PROJECT_NAME.to_uppercase());
				let dir = if let Ok(path_str) = std::env::var(&organize_config_dir) {
					PathBuf::from(path_str)
				} else {
					dirs::config_dir()
						.map(|dir| dir.join(PROJECT_NAME))
						.unwrap_or_else(|| panic!("could not find config directory, please set {}", organize_config_dir))
				};
				dir.join("config.toml")
			}
		}
	}
}

// #[cfg(test)]
// mod tests {
// 	use std::sync::LazyLock;

// 	use super::ConfigBuilder;
// 	use itertools::Itertools;
// 	use toml::toml;

// 	static TOML: LazyLock<toml::Table> = LazyLock::new(|| {
// 		toml! {

// 				[[rules]]
// 				id = "test-rule-1"
// 				tags = ["tag1"]

// 				actions = []
// 				filters = []
// 				folders = []

// 				[[rules]]
// 				id = "test-rule-2"
// 				tags = ["tag2"]

// 				actions = []
// 				filters = []
// 				folders = []

// 				[[rules]]
// 				id = "test-rule-3"
// 				tags = ["tag3"]

// 				actions = []
// 				filters = []
// 				folders = []

// 				[[rules]]
// 				tags = ["tag3"]

// 				actions = []
// 				filters = []
// 				folders = []

// 				[[rules]]
// 				actions = []
// 				filters = []
// 				folders = []

// 		}
// 	});

// 	static CONFIG: LazyLock<ConfigBuilder> = LazyLock::new(|| toml::from_str(&TOML.to_string()).unwrap());

// 	#[test]
// 	fn filter_rules_by_tag_positive() {
// 		let found_rules = CONFIG.filter_rules_by_tag(["tag2"]).iter().map(|&r| r.clone()).collect_vec();
// 		let expected_rules = CONFIG.rules.get(1..=1).unwrap();
// 		assert_eq!(found_rules, expected_rules)
// 	}
// 	#[test]
// 	fn filter_rules_by_tag_negative() {
// 		let found_rules = CONFIG.filter_rules_by_tag(["!tag2"]).iter().copied().collect_vec();
// 		let expected_rules = vec![CONFIG.rules.first().unwrap(), CONFIG.rules.get(2).unwrap(), CONFIG.rules.get(3).unwrap()];
// 		assert_eq!(found_rules, expected_rules)
// 	}
// 	#[test]
// 	fn filter_rules_by_tag_multiple_positive() {
// 		let found_rules = CONFIG
// 			.filter_rules_by_tag(["tag2", "tag1"])
// 			.iter()
// 			.copied()
// 			.cloned()
// 			.collect_vec();
// 		let expected_rules = CONFIG.rules.get(..=1).unwrap();
// 		assert_eq!(found_rules, expected_rules)
// 	}
// 	#[test]
// 	fn filter_rules_by_tag_multiple_negative() {
// 		let found_rules = CONFIG
// 			.filter_rules_by_tag(["!tag2", "!tag1"])
// 			.iter()
// 			.copied()
// 			.cloned()
// 			.collect_vec();
// 		let expected_rules = CONFIG.rules.get(2..=3).unwrap();
// 		assert_eq!(found_rules, expected_rules)
// 	}
// 	#[test]
// 	fn filter_rules_by_tag_multiple_mixed() {
// 		let found_rules = CONFIG
// 			.filter_rules_by_tag(["tag2", "!tag1"])
// 			.iter()
// 			.copied()
// 			.cloned()
// 			.collect_vec();
// 		let expected_rules = CONFIG.rules.get(1..=3).unwrap();
// 		assert_eq!(found_rules, expected_rules)
// 	}
// 	#[test]
// 	fn filter_rules_by_id_positive() {
// 		let found_rules = CONFIG.filter_rules_by_id(["test-rule-1"]).iter().copied().collect_vec();
// 		let expected_rules = vec![CONFIG.rules.first().unwrap()];
// 		assert_eq!(found_rules, expected_rules)
// 	}
// 	#[test]
// 	fn filter_rules_by_id_negative() {
// 		let found_rules = CONFIG
// 			.filter_rules_by_id(["!test-rule-1"])
// 			.iter()
// 			.map(|r| (*r).clone())
// 			.collect_vec();
// 		let expected_rules = CONFIG.rules.get(1..=2).unwrap();
// 		assert_eq!(found_rules, expected_rules)
// 	}
// 	#[test]
// 	fn filter_rules_by_id_multiple_positive() {
// 		let found_rules = CONFIG
// 			.filter_rules_by_id(["test-rule-1", "test-rule-2"])
// 			.iter()
// 			.map(|&r| r.clone())
// 			.collect_vec();
// 		let expected_rules = CONFIG.rules.get(0..=1).unwrap();
// 		assert_eq!(found_rules, expected_rules)
// 	}
// 	#[test]
// 	fn filter_rules_by_id_multiple_negative() {
// 		let found_rules = CONFIG
// 			.filter_rules_by_id(["!test-rule-1", "!test-rule-2"])
// 			.iter()
// 			.copied()
// 			.collect_vec();
// 		let expected_rules = vec![CONFIG.rules.get(2).unwrap()];
// 		assert_eq!(found_rules, expected_rules)
// 	}
// 	#[test]
// 	fn filter_rules_by_id_multiple_mixed() {
// 		let found_rules = CONFIG
// 			.filter_rules_by_id(["test-rule-1", "!test-rule-2"])
// 			.iter()
// 			.copied()
// 			.collect_vec();
// 		let expected_rules = vec![CONFIG.rules.first().unwrap(), CONFIG.rules.get(2).unwrap()];
// 		assert_eq!(found_rules, expected_rules)
// 	}
// }
