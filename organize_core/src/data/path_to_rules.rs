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
		self.0.get_key_value(key).map_or_else(|| {
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
		}, |(k, v)| (k.clone().clone(), v))
	}

	pub fn get(&self, key: &PathBuf) -> &Vec<(usize, usize)> {
		self.get_key_value(key).1
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::data::config::folders::Folder;
	use crate::data::config::Rule;
	use crate::data::options::Options;
	use crate::utils::DefaultOpt;
	use dirs::home_dir;
	use std::convert::TryFrom;

	#[test]
	fn test_new() {
		let home = home_dir().unwrap();
		let test1 = home.join("test1");
		if !test1.exists() {
			std::fs::create_dir_all(&test1).unwrap();
		}
		let test2 = home.join("test2");
		if !test2.exists() {
			std::fs::create_dir_all(&test2).unwrap();
		}
		let test3 = home.join("test3");
		if !test3.exists() {
			std::fs::create_dir_all(&test3).unwrap();
		}

		let rules = vec![
			Rule {
				folders: vec![
					Folder::try_from(test1.clone()).unwrap(),
					Folder::try_from(test2.clone()).unwrap(),
				],
				..Default::default()
			},
			Rule {
				folders: vec![
					Folder::try_from(test3.clone()).unwrap(),
					Folder::try_from(test1.clone()).unwrap(),
				],
				..Default::default()
			},
		];
		let config = Config {
			rules,
			defaults: Options::default_none(),
		};

		std::fs::remove_dir(&test1).unwrap();
		std::fs::remove_dir(&test2).unwrap();
		std::fs::remove_dir(&test3).unwrap();

		let value = PathToRules::new(&config).0;
		let mut expected = HashMap::new();
		expected.insert(&test1, vec![(0, 0), (1, 1)]);
		expected.insert(&test2, vec![(0, 1)]);
		expected.insert(&test3, vec![(1, 0)]);

		assert_eq!(value, expected)
	}
}
