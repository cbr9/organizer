use serde::{Deserialize, Serialize};
use std::path::Path;
use walkdir::WalkDir;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(transparent)]
pub struct MaxDepth(pub u64);

impl MaxDepth {
	pub fn to_walker<T: AsRef<Path>>(&self, path: T) -> WalkDir {
		let max_depth = if path.as_ref() == dirs_next::home_dir().unwrap() {
			1
		} else if self.0 == 0 {
			f64::INFINITY as u64
		} else {
			self.0
		};

		WalkDir::new(path).min_depth(1).max_depth(max_depth as usize)
	}
}

impl Default for MaxDepth {
	fn default() -> Self {
		MaxDepth(1)
	}
}
