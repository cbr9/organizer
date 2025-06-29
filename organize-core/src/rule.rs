use std::collections::HashSet;

use itertools::Itertools;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
	action::Action,
	common::enabled,
	filter::Filter,
	grouper::Grouper,
	options::OptionsBuilder,
	sorter::Sorter,
	storage::Location,
	templates::prelude::*,
};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Rule {
	pub id: Option<String>,
	#[serde(default)]
	pub tags: Vec<String>,
	pub pipeline: Vec<Stage>,
	pub locations: Vec<Box<dyn Location>>,
	#[serde(flatten)]
	pub options: OptionsBuilder,
	#[serde(default = "enabled")]
	pub enabled: bool,
}

impl Rule {
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

#[derive(Debug, Serialize, PartialEq, Eq, Clone)]
pub enum Stage {
	Action(Box<dyn Action>),
	Filter(Box<dyn Filter>),
	Grouper(Box<dyn Grouper>),
	Sorter(Box<dyn Sorter>),
}

impl<'de> Deserialize<'de> for Stage {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		#[derive(Deserialize, Debug, Default)]
		#[serde(deny_unknown_fields)]
		struct StageHelper {
			#[serde(default, skip_serializing_if = "Option::is_none")]
			action: Option<Box<dyn Action>>,
			#[serde(default, skip_serializing_if = "Option::is_none")]
			filter: Option<Box<dyn Filter>>,
			#[serde(default, skip_serializing_if = "Option::is_none", rename = "group-by")]
			grouper: Option<Box<dyn Grouper>>,
			#[serde(default, skip_serializing_if = "Option::is_none", rename = "sort-by")]
			sorter: Option<Box<dyn Sorter>>,
		}

		let helper = StageHelper::deserialize(deserializer)?;
		let count = helper.action.is_some() as u8 + helper.filter.is_some() as u8 + helper.grouper.is_some() as u8 + helper.sorter.is_some() as u8;

		if count != 1 {
			return Err(serde::de::Error::custom(
				"A stage must have exactly one key: 'action', 'filter', 'folders', 'group-by', or 'sort-by'",
			));
		}

		if let Some(action) = helper.action {
			Ok(Stage::Action(action))
		} else if let Some(filter) = helper.filter {
			Ok(Stage::Filter(filter))
		} else if let Some(grouper) = helper.grouper {
			Ok(Stage::Grouper(grouper))
		} else if let Some(sorter) = helper.sorter {
			Ok(Stage::Sorter(sorter))
		} else {
			unreachable!();
		}
	}
}
