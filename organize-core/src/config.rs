


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
