use crate::utils::DefaultOpt;
use serde::{Deserialize, Serialize};
use std::path::Path;
use walkdir::WalkDir;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(transparent)]
pub struct Recursive {
	pub depth: Option<u16>, // if depth is some, enabled should be true
}

impl DefaultOpt for Recursive {
	fn default_none() -> Self {
		Self { depth: None }
	}

	fn default_some() -> Self {
		Self { depth: Some(1) }
	}
}

impl Recursive {
	pub fn to_walker<T: AsRef<Path>>(&self, path: T) -> WalkDir {
		match self.depth.unwrap() {
			0 => WalkDir::new(path).min_depth(1),
			other => WalkDir::new(path).min_depth(1).max_depth(other as usize),
		}
	}

	pub fn as_mode(&self) -> notify::RecursiveMode {
		if self.is_recursive() {
			return notify::RecursiveMode::Recursive;
		} else {
			return notify::RecursiveMode::NonRecursive;
		}
	}

	pub fn is_recursive(&self) -> bool {
		self.depth.map(|depth| depth == 0 || depth > 1).unwrap_or_default()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn is_recursive() {
		assert!(!Recursive { depth: Some(1) }.is_recursive());
		assert!(Recursive { depth: Some(0) }.is_recursive());
		assert!(Recursive { depth: Some(3) }.is_recursive());
	}
}
