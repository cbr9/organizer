use std::{collections::HashSet, sync::Arc};

use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::templates::Templater;

use super::{
	actions::Action,
	filters::Filter,
	folders::{Folder, FolderBuilder},
	options::OptionsBuilder,
	variables::Variable,
};

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuleBuilder {
	pub id: Option<String>,
	#[serde(default)]
	pub tags: HashSet<String>,
	pub actions: Vec<Box<dyn Action>>,
	#[serde(default)]
	pub filters: Vec<Box<dyn Filter>>,
	pub folders: Vec<FolderBuilder>,
	#[serde(flatten)]
	pub options: OptionsBuilder,
	#[serde(default)]
	pub variables: Vec<Box<dyn Variable>>,
}

impl RuleBuilder {
	pub fn build(self, index: usize, defaults: &OptionsBuilder, template_engine: &mut Templater) -> anyhow::Result<Rule> {
		let folders = self
			.folders
			.iter()
			.cloned()
			.enumerate()
			.filter_map(|(idx, builder)| builder.build(idx, defaults, &self.options, template_engine).ok()) // Pass this rule's options builder
			.collect_vec();

		Ok(Rule {
			index,
			id: self.id,
			tags: self.tags,
			actions: self.actions,
			filters: self.filters,
			variables: self.variables,
			folders, // Contains fully compiled Folders, each with its own Options
		})
	}

	/// Checks if a single rule should be run based on pre-compiled sets of chosen tags.
	pub fn matches_tags(&self, positive_tags: &HashSet<String>, negative_tags: &HashSet<String>) -> bool {
		// Rule is disqualified if it contains any of the negative tags.
		if self.tags.iter().any(|tag| negative_tags.contains(tag.as_str())) {
			return false;
		}

		// If positive tags are specified, the rule must have at least one of them.
		// If no positive tags are specified, this condition is met.
		positive_tags.is_empty() || self.tags.iter().any(|tag| positive_tags.contains(tag.as_str()))
	}

	/// Checks if a single rule should be run based on pre-compiled sets of chosen IDs.
	pub fn matches_ids(&self, positive_ids: &HashSet<String>, negative_ids: &HashSet<String>) -> bool {
		let rule_id = self.id.as_deref();

		// Rule is disqualified if its ID is one of the negative IDs.
		if let Some(id) = rule_id {
			if negative_ids.contains(id) {
				return false;
			}
		}

		// If positive IDs are specified, the rule's ID must be one of them.
		// If no positive IDs are specified, this condition is met.
		positive_ids.is_empty() || rule_id.is_some_and(|id| positive_ids.contains(id))
	}
}

#[derive(Debug, PartialEq, Clone)]
pub struct Rule {
	pub index: usize,
	pub id: Option<String>,
	pub tags: HashSet<String>,
	pub actions: Vec<Box<dyn Action>>,
	pub filters: Vec<Box<dyn Filter>>,
	pub variables: Vec<Box<dyn Variable>>,
	pub folders: Vec<Folder>,
}
