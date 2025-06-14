use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::utils::DefaultOpt;

use super::{actions::Action, filters::Filter, folders::Folders, options::Options, variables::Variable};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
	pub id: Option<String>,
	#[serde(default)]
	pub tags: HashSet<String>,
	#[serde(default)]
	pub r#continue: bool,
	pub filters: Vec<Box<dyn Filter>>,
	pub actions: Vec<Box<dyn Action>>,
	pub folders: Folders,
	#[serde(default = "Options::default_none")]
	pub options: Options,
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
			filters: vec![],
			folders: vec![],
			options: Options::default_none(),
		}
	}
}
