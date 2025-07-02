use std::{collections::HashMap, sync::Arc};

use crate::resource::Resource;

/// Represents a batch of files that have been grouped by one or more criteria.
/// This is the primary data structure that flows between pipeline stages.
#[derive(Debug, Clone)]
pub struct Batch {
	pub name: String,
	pub files: Vec<Arc<Resource>>,
	pub context: HashMap<String, String>,
}

impl Batch {
	pub fn new() -> Self {
		Self {
			name: String::new(),
			files: Vec::new(),
			context: HashMap::new(),
		}
	}

	pub fn initial(files: Vec<Arc<Resource>>) -> Self {
		Self {
			name: String::new(),
			files,
			context: HashMap::new(),
		}
	}
}

impl Default for Batch {
	fn default() -> Self {
		Self::new()
	}
}
