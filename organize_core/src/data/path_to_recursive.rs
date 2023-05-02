use crate::data::{options::recursive::Recursive, Data};

use std::{
	collections::{
		hash_map::{Iter, Keys},
		HashMap,
	},
	path::Path,
};

#[derive(Debug, Eq, PartialEq)]
pub struct PathToRecursive<'a>(HashMap<&'a Path, Recursive>);

impl<'a> PathToRecursive<'a> {
	pub fn new(data: &'a Data) -> Self {
		let mut map = HashMap::with_capacity(data.config.rules.len());
		data.config.rules.iter().enumerate().for_each(|(i, rule)| {
			rule.folders.iter().enumerate().for_each(|(j, folder)| {
				let depth = *data.get_recursive_depth(i, j);
				map.entry(folder.path.as_path())
					.and_modify(|entry: &mut Recursive| {
						if let Some(curr_depth) = entry.depth {
							if curr_depth != 0 && (depth == 0 || depth > curr_depth) {
								// take the greatest depth, except if it equals 0 or the current depth is already 0
								entry.depth = Some(depth);
							}
						}
					})
					.or_insert(Recursive { depth: Some(depth) });
			})
		});
		map.shrink_to_fit();
		Self(map)
	}

	pub fn keys(&self) -> Keys<'_, &'a Path, Recursive> {
		self.0.keys()
	}

	pub fn iter(&self) -> Iter<'_, &'a Path, Recursive> {
		self.0.iter()
	}

	pub fn get<T: AsRef<Path>>(&self, key: T) -> Option<&Recursive> {
		self.0.get(key.as_ref())
	}

	pub fn insert<T: AsRef<Path>>(&mut self, key: &'a T, value: Recursive) -> Option<Recursive> {
		self.0.insert(key.as_ref(), value)
	}
}
