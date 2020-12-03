use crate::data::Data;
use std::{
	collections::{hash_map::Keys, HashMap},
	path::PathBuf,
};

pub struct PathToRules<'a>(HashMap<&'a PathBuf, Vec<(usize, usize)>>);

impl<'a> PathToRules<'a> {
	pub fn new(data: &'a Data) -> Self {
		let mut map = HashMap::with_capacity(data.config.rules.len());
		data.config.rules.iter().enumerate().for_each(|(i, rule)| {
			rule.folders.iter().enumerate().for_each(|(j, folder)| {
				let path = &folder.path;
				if !map.contains_key(path) {
					map.insert(path, Vec::new());
				}
				map.get_mut(path).unwrap().push((i, j));
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
			let mut paths = Vec::new();
			for i in 0..components.len() {
				let slice = components[0..i].iter().map(|comp| comp.as_os_str().to_string_lossy()).collect::<Vec<_>>();
				let str: String = slice.join(&std::path::MAIN_SEPARATOR.to_string());
				paths.push(PathBuf::from(str.replace("//", "/")))
			}
			let path = paths
				.iter()
				.rev()
				.find_map(|path| if self.0.contains_key(path) { Some(path) } else { None })
				.unwrap();
			self.0.get(path).unwrap()
		})
	}
}
