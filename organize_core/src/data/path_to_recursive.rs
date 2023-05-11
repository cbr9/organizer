use derive_more::Deref;

use crate::data::{options::recursive::Recursive, Data};

use std::{collections::HashMap, path::PathBuf};

#[derive(Deref, Debug, Eq, PartialEq)]
pub struct PathToRecursive(HashMap<PathBuf, Recursive>);

impl PathToRecursive {
	pub fn new(data: Data) -> Self {
		let mut map = HashMap::with_capacity(data.config.rules.len());
		data.config.rules.iter().enumerate().for_each(|(i, rule)| {
			rule.folders.iter().enumerate().for_each(|(j, folder)| {
				let depth = *data.get_recursive_depth(i, j);
				map.entry(folder.path.to_path_buf())
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
}
