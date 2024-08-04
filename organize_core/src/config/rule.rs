use std::collections::HashSet;

use serde::Deserialize;

use crate::utils::DefaultOpt;

use super::{actions::Action, filters::Filters, folders::Folders, options::FolderOptions, variables::Variable};

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Rule {
	pub id: Option<String>,
	#[serde(default)]
	pub tags: HashSet<String>,
	#[serde(default)]
	pub r#continue: bool,
	pub actions: Vec<Action>,
	pub filters: Filters,
	pub folders: Folders,
	#[serde(default = "FolderOptions::default_none")]
	pub options: FolderOptions,
	#[serde(default)]
	pub variables: Vec<Variable>,
}

impl Default for Rule {
	fn default() -> Self {
		Self {
			id: None,
			tags: HashSet::new(),
			r#continue: false,
			variables: vec![],
			actions: vec![],
			filters: Filters(vec![]),
			folders: vec![],
			options: FolderOptions::default_none(),
		}
	}
}
