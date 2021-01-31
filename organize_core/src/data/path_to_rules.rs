use crate::data::config::Config;
use std::{
	collections::{hash_map::Keys, HashMap},
	path::{Path, PathBuf},
};

pub struct PathToRules<'a>(HashMap<&'a PathBuf, Vec<(usize, usize)>>);

impl<'a> PathToRules<'a> {
	pub fn new(config: &'a Config) -> Self {
		let mut map = HashMap::with_capacity(config.rules.len()); // there will be at least one folder per rule
		config.rules.iter().enumerate().for_each(|(i, rule)| {
			rule.folders.iter().enumerate().for_each(|(j, folder)| {
				map.entry(&folder.path).or_insert_with(Vec::new).push((i, j));
			})
		});
		map.shrink_to_fit();
		Self(map)
	}

	pub fn keys(&self) -> Keys<'_, &'a PathBuf, Vec<(usize, usize)>> {
		self.0.keys()
	}

	pub fn get_key_value<T: AsRef<Path>>(&self, key: T) -> Option<(&&PathBuf, &Vec<(usize, usize)>)> {
		key.as_ref()
			.ancestors()
			.find_map(|ancestor| self.0.get_key_value(&ancestor.to_path_buf()))
	}

	pub fn get<T: AsRef<Path>>(&self, key: T) -> Option<&Vec<(usize, usize)>> {
		self.get_key_value(key).map(|(_, value)| value)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		data::{
			config::{folders::Folder, Rule},
			options::Options,
		},
		utils::DefaultOpt,
	};
	use std::path::Path;

	#[test]
	fn test_key_value() {
		let downloads = "$HOME/Downloads";
		let docs = "$HOME/Documents";
		let pdfs = Path::new(docs).join("pdfs");
		let torrents = Path::new(downloads).join("torrents");

		let config = Config {
			rules: vec![Rule {
				folders: vec![
					Folder {
						path: downloads.into(),
						options: Options::default_none(),
					},
					Folder {
						path: docs.into(),
						options: Options::default_none(),
					},
				],
				..Rule::default()
			}],
			defaults: Options::default_none(),
		};

		let path_to_rules = PathToRules::new(&config);
		let (key, _) = path_to_rules.get_key_value(&torrents).unwrap();
		assert_eq!(key, &&PathBuf::from(downloads)); // torrents is not in config but its direct ancestor, `downloads`, is
		let (key, _) = path_to_rules.get_key_value(&pdfs).unwrap();
		assert_eq!(key, &&PathBuf::from(docs))
	}

	#[test]
	fn test_new() {
		let test1 = PathBuf::from("test1");
		let test2 = PathBuf::from("test2");
		let test3 = PathBuf::from("test3");

		let rules = vec![
			Rule {
				// 0
				folders: vec![
					Folder {
						// 0
						path: test1.clone(),
						options: Options::default_none(),
					},
					Folder {
						// 1
						path: test2.clone(),
						options: Options::default_none(),
					},
				],
				..Default::default()
			},
			Rule {
				// 1
				folders: vec![
					Folder {
						// 0
						path: test3.clone(),
						options: Options::default_none(),
					},
					Folder {
						// 1
						path: test1.clone(),
						options: Options::default_none(),
					},
				],
				..Default::default()
			},
			Rule {
				// 2
				folders: vec![Folder {
					// 0
					path: test3.clone(),
					options: Options::default_none(),
				}],
				..Default::default()
			},
		];
		let config = Config {
			rules,
			defaults: Options::default_none(),
		};

		let value = PathToRules::new(&config).0;
		let mut expected = HashMap::new();
		expected.insert(&test1, vec![(0, 0), (1, 1)]);
		expected.insert(&test2, vec![(0, 1)]);
		expected.insert(&test3, vec![(1, 0), (2, 0)]);

		assert_eq!(value, expected)
	}
}
