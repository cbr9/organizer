use serde::Deserialize;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct MaxDepth(f64);

impl MaxDepth {
	pub fn to_walker<T: AsRef<Path>>(&self, path: T) -> WalkDir {
		let max_depth = if path.as_ref() == dirs::home_dir().unwrap() { 1.0 } else { self.0 };
		WalkDir::new(path)
			.min_depth(1)
			.max_depth(max_depth as usize)
			.contents_first(true)
	}
}

impl Default for MaxDepth {
	fn default() -> Self {
		MaxDepth(1.0)
	}
}
