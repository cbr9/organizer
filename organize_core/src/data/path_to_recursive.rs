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

	pub fn get(&self, key: &Path) -> Option<&Recursive> {
		self.0.get(key)
	}

	pub fn insert(&mut self, key: &'a Path, value: Recursive) -> Option<Recursive> {
		self.0.insert(key, value)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		data::{
			config::{folders::Folder, Config, Rule},
			options::{recursive::Recursive, Options},
			settings::Settings,
		},
		utils::DefaultOpt,
	};

	#[test]
	fn new() {
		let downloads = "$HOME/Downloads";
		let documents = "$HOME/Documents";
		let data = Data {
			defaults: Options::default_some(),
			settings: Settings::default_some(),
			config: Config {
				rules: vec![
					Rule {
						folders: vec![
							Folder {
								path: downloads.into(),
								options: Options {
									recursive: Recursive { depth: Some(3) },
									..Options::default_none()
								},
							},
							Folder {
								path: documents.into(),
								options: Options {
									recursive: Recursive { depth: None },
									..DefaultOpt::default_none()
								},
							},
						],
						..Rule::default()
					},
					Rule {
						folders: vec![
							Folder {
								path: downloads.into(),
								options: Options {
									recursive: Recursive { depth: Some(0) },
									..DefaultOpt::default_none()
								},
							},
							Folder {
								path: documents.into(),
								options: Options {
									recursive: Recursive { depth: Some(5) },
									..DefaultOpt::default_none()
								},
							},
						],
						..Rule::default()
					},
					Rule {
						folders: vec![Folder {
							path: downloads.into(),
							options: Options {
								recursive: Recursive { depth: Some(4) },
								..DefaultOpt::default_none()
							},
						}],
						..Rule::default()
					},
				],
				defaults: Options::default_none(),
			},
		};
		let mut expected = HashMap::new();
		expected.insert(Path::new(downloads), Recursive { depth: Some(0) });
		expected.insert(Path::new(documents), Recursive { depth: Some(5) });
		let path_to_recursive = PathToRecursive::new(&data);
		assert_eq!(path_to_recursive.0, expected);
	}
}
