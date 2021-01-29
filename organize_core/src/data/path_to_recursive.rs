use crate::data::Data;
use notify::RecursiveMode;
use std::{
	collections::{
		hash_map::{Iter, Keys},
		HashMap,
	},
	path::Path,
};

#[derive(Debug, Eq, PartialEq)]
pub struct PathToRecursive<'a>(HashMap<&'a Path, (RecursiveMode, Option<u16>)>);

impl<'a> PathToRecursive<'a> {
	pub fn new(data: &'a Data) -> Self {
		let mut map = HashMap::with_capacity(data.config.rules.len());
		data.config.rules.iter().enumerate().for_each(|(i, rule)| {
			rule.folders.iter().enumerate().for_each(|(j, folder)| {
				let recursive = if *data.get_recursive_enabled(i, j) {
					RecursiveMode::Recursive
				} else {
					RecursiveMode::NonRecursive
				};
				map.entry(folder.path.as_path())
					.and_modify(|entry: &mut (RecursiveMode, Option<u16>)| {
						if recursive == RecursiveMode::Recursive && entry.0 == RecursiveMode::NonRecursive {
							let depth = data.get_recursive_depth(i, j);
							*entry = (recursive, Some(*depth));
						}
					})
					.or_insert((recursive, None));
			})
		});
		map.shrink_to_fit();
		Self(map)
	}

	pub fn keys(&self) -> Keys<'_, &'a Path, (RecursiveMode, Option<u16>)> {
		self.0.keys()
	}

	pub fn iter(&self) -> Iter<'_, &'a Path, (RecursiveMode, Option<u16>)> {
		self.0.iter()
	}

	pub fn get(&self, key: &Path) -> Option<&(RecursiveMode, Option<u16>)> {
		self.0.get(key)
	}

	pub fn insert(&mut self, key: &'a Path, value: (RecursiveMode, Option<u16>)) -> Option<(RecursiveMode, Option<u16>)> {
		self.0.insert(key, value)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::data::config::folders::Folder;
	use crate::data::config::{Config, Rule};
	use crate::data::options::recursive::Recursive;
	use crate::data::options::Options;
	use crate::data::settings::Settings;
	use crate::utils::DefaultOpt;

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
								options: Options::default_none(),
							},
							Folder {
								path: documents.into(),
								options: Options {
									recursive: Recursive {
										enabled: Some(false),
										depth: None,
									},
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
								recursive: Recursive {
									enabled: Some(true),
									depth: Some(1),
								},
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
		expected.insert(Path::new(downloads), (RecursiveMode::Recursive, Some(1)));
		expected.insert(Path::new(documents), (RecursiveMode::NonRecursive, None));
		let path_to_recursive = PathToRecursive::new(&data);
		assert_eq!(path_to_recursive.0, expected);
	}
}
