use serde::{Deserialize, Serialize};
use std::path::Path;
use walkdir::WalkDir;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(transparent)]
pub struct Recursive(pub u64);

impl Recursive {
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

	pub fn type_(&self) -> notify::RecursiveMode {
		if self.0 != 1 {
			notify::RecursiveMode::Recursive
		} else {
			notify::RecursiveMode::NonRecursive
		}
	}
}

impl Default for Recursive {
	fn default() -> Self {
		Recursive(1)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	// use notify::RecursiveMode::*;

	#[test]
	fn is_recursive() {
		assert!(Recursive(1).type_() == notify::RecursiveMode::NonRecursive);
		assert!(Recursive(0).type_() == notify::RecursiveMode::Recursive);
		assert!(Recursive(2).type_() == notify::RecursiveMode::Recursive);
	}
}
