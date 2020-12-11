use crate::{data::Data};
use notify::RecursiveMode;
use std::{
	collections::{
		hash_map::{Iter, Keys},
		HashMap,
	},
	path::Path,
};

pub struct PathToRecursive<'a>(HashMap<&'a Path, RecursiveMode>);

impl<'a> PathToRecursive<'a> {
	pub fn new(data: &'a Data) -> Self {
		let mut map = HashMap::with_capacity(data.config.rules.len());
		data.config.rules.iter().enumerate().for_each(|(i, rule)| {
			rule.folders.iter().enumerate().for_each(|(j, folder)| {
				let recursive = if *data.get_recursive(i, j) {
					RecursiveMode::Recursive
				} else {
					RecursiveMode::NonRecursive
				};
				match map.get(folder.path.as_path()) {
					None => {
						map.insert(folder.path.as_path(), recursive);
					}
					Some(value) => {
						if recursive == RecursiveMode::Recursive && value == &RecursiveMode::NonRecursive {
							map.insert(folder.path.as_path(), recursive);
						}
					}
				}
			})
		});
		map.shrink_to_fit();
		Self(map)
	}

	pub fn keys(&self) -> Keys<'_, &'a Path, RecursiveMode> {
		self.0.keys()
	}

	pub fn iter(&self) -> Iter<'_, &'a Path, RecursiveMode> {
		self.0.iter()
	}

	pub fn get(&self, key: &Path) -> Option<&RecursiveMode> {
		self.0.get(key)
	}

	pub fn insert(&mut self, key: &'a Path, value: RecursiveMode) -> Option<RecursiveMode> {
		self.0.insert(key, value)
	}
}
