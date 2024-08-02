use serde::Deserialize;

use crate::utils::DefaultOpt;

use super::{actions::Action, filters::Filters, folders::Folders, options::FolderOptions};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Rule {
	pub name: Option<String>,
	#[serde(default)]
	pub tags: Vec<String>,
	pub actions: Vec<Action>,
	pub filters: Filters,
	pub folders: Folders,
	#[serde(default = "FolderOptions::default_none")]
	pub options: FolderOptions,
}

impl Default for Rule {
	fn default() -> Self {
		Self {
			name: None,
			tags: vec![],
			actions: vec![],
			filters: Filters(vec![]),
			folders: vec![],
			options: FolderOptions::default_none(),
		}
	}
}
