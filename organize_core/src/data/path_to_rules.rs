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

	pub fn get(&self, key: &PathBuf) -> &Vec<(usize, usize)> {
		self.0.get(key).unwrap_or_else(|| {
			// if the path is some subdirectory not represented in the hashmap
			let components = key.components().collect::<Vec<_>>();
			(0..components.len())
				.map(|i| PathBuf::from_iter(&components[0..i]))
				.rev()
				.find(|path| self.0.contains_key(&path))
				.map(|path| self.0.get(&path).unwrap())
				.unwrap()
		})
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
		let download_dir = home.join("Downloads");
		if !download_dir.exists() {
			std::fs::create_dir_all(&download_dir).unwrap();
		}
		let document_dir = home.join("Documents");
		if !document_dir.exists() {
			std::fs::create_dir_all(&document_dir).unwrap();
		}
		let picture_dir = home.join("Pictures");
		if !picture_dir.exists() {
			std::fs::create_dir_all(&picture_dir).unwrap();
		}

		let rules = vec![
			Rule {
				folders: vec![
					Folder::try_from(download_dir.clone()).unwrap(),
					Folder::try_from(document_dir.clone()).unwrap(),
				],
				..Default::default()
			},
			Rule {
				folders: vec![
					Folder::try_from(picture_dir.clone()).unwrap(),
					Folder::try_from(download_dir.clone()).unwrap(),
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
		expected.insert(&download_dir, vec![(0, 0), (1, 1)]);
		expected.insert(&document_dir, vec![(0, 1)]);
		expected.insert(&picture_dir, vec![(1, 0)]);

		assert_eq!(value, expected)
	}
}
