use crate::data::config::Config;
use std::iter::FromIterator;
use std::{
	collections::{hash_map::Keys, HashMap},
	path::PathBuf,
};

pub struct PathToRules<'a>(HashMap<&'a PathBuf, Vec<(usize, usize)>>);

impl<'a> PathToRules<'a> {
	pub fn new(config: &'a Config) -> Self {
		let mut map = HashMap::with_capacity(config.rules.len()); // there will be at least one folder per rule
		config.rules.iter().enumerate().for_each(|(i, rule)| {
			rule.folders.iter().enumerate().for_each(|(j, folder)| {
				let path = &folder.path;
				match map.get_mut(path) {
					None => {
						map.insert(path, vec![(i, j)]);
					}
					Some(vec) => {
						vec.push((i, j));
					}
				};
			})
		});
		map.shrink_to_fit();
		Self(map)
	}

	pub fn keys(&self) -> Keys<'_, &'a PathBuf, Vec<(usize, usize)>> {
		self.0.keys()
	}

	pub fn get_key_value(&self, key: &PathBuf) -> (PathBuf, &Vec<(usize, usize)>) {
		self.0.get_key_value(key).map_or_else(
			|| {
				// if the path is some subdirectory not represented in the hashmap
				let components = key.components().collect::<Vec<_>>();
				(0..components.len())
					.map(|i| PathBuf::from_iter(&components[0..i]))
					.rev()
					.find(|path| self.0.contains_key(&path))
					.map(|path| {
						let value = self.0.get(&path).unwrap();
						(path, value)
					})
					.unwrap()
			},
			|(k, v)| ((*k).clone(), v),
		)
	}

	pub fn get(&self, key: &PathBuf) -> &Vec<(usize, usize)> {
		self.get_key_value(key).1
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::data::config::actions::Actions;
	use crate::data::config::filters::Filters;
	use crate::data::config::folders::Folder;
	use crate::data::config::Rule;
	use crate::data::options::Options;
	use crate::utils::DefaultOpt;
	use std::path::Path;

	#[test]
	fn test_key_value() {
		let downloads = "$HOME/Downloads";
		let docs = "$HOME/Documents";
		let pdfs = Path::new(docs).join("pdfs");
		let torrents = Path::new(downloads).join("torrents");
		let config = Config {
			rules: vec![Rule {
				actions: Actions(vec![]),
				filters: Filters { inner: vec![] },
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
				options: Options::default_none(),
			}],
			defaults: Options::default_none(),
		};
		let path_to_rules = PathToRules::new(&config);
		let (key, _) = path_to_rules.get_key_value(&torrents);
		assert_eq!(key, Path::new(downloads));
		let (key, _) = path_to_rules.get_key_value(&pdfs);
		assert_eq!(key, Path::new(docs))
	}

	#[test]
	fn test_new() {
		let test1 = PathBuf::from("test1");
		let test2 = PathBuf::from("test2");
		let test3 = PathBuf::from("test3");

		let rules = vec![
			Rule {
				folders: vec![
					Folder {
						path: test1.clone(),
						options: Options::default_none(),
					},
					Folder {
						path: test2.clone(),
						options: Options::default_none(),
					},
				],
				..Default::default()
			},
			Rule {
				folders: vec![
					Folder {
						path: test3.clone(),
						options: Options::default_none(),
					},
					Folder {
						path: test1.clone(),
						options: Options::default_none(),
					},
				],
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
		expected.insert(&test3, vec![(1, 0)]);

		assert_eq!(value, expected)
	}
}
