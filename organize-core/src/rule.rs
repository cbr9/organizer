use std::{path::PathBuf, sync::Arc};

use itertools::Itertools;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
	action::{Action, ActionBuilder},
	context::ExecutionContext,
	errors::Error,
	filter::Filter,
	folder::{Location, LocationBuilder},
	grouper::Grouper,
	selector::Selector,
	sorter::Sorter,
};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct RuleMetadata {
	pub name: Option<String>,
	pub description: Option<String>,
	#[serde(default)]
	pub tags: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuleBuilder {
	#[serde(flatten)]
	pub metadata: RuleMetadata,
	#[serde(rename = "stage")]
	pub pipeline: Vec<StageBuilder>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
	pub metadata: Arc<RuleMetadata>,
	pub pipeline: Vec<Stage>,
}

async fn load_rule_builder_from_path(path: &std::path::Path) -> Result<RuleBuilder, anyhow::Error> {
	let content = tokio::fs::read_to_string(path).await?;
	let builder: RuleBuilder = toml::from_str(&content)?;
	Ok(builder)
}

impl RuleBuilder {
	pub async fn build(self, ctx: &ExecutionContext<'_>) -> Result<Rule, Error> {
		let mut final_pipeline = Vec::new();
		let main_meta = Arc::new(self.metadata);
		let mut processing_stack: Vec<(StageBuilder, Arc<RuleMetadata>)> = self
			.pipeline
			.into_iter()
			.map(|builder| (builder, main_meta.clone()))
			.rev()
			.collect();

		while let Some((builder, meta)) = processing_stack.pop() {
			match builder {
				StageBuilder::Compose(path) => {
					let composed_builder = load_rule_builder_from_path(&path).await?;
					let composed_meta = Arc::new(composed_builder.metadata);
					for stage_builder in composed_builder.pipeline.into_iter().rev() {
						processing_stack.push((stage_builder, composed_meta.clone()));
					}
				}
				// The logic to build the final Stage enum now changes slightly
				other_builder => {
					let stage_enum = other_builder.build(ctx, meta).await?;
					final_pipeline.push(stage_enum);
				}
			}
		}

		Ok(Rule {
			metadata: main_meta.clone(),
			pipeline: final_pipeline,
		})
	}
}

#[derive(Debug, Serialize, PartialEq, Eq, Clone)]
pub enum StageBuilder {
	Search(LocationBuilder),
	Compose(PathBuf),
	Action(Box<dyn ActionBuilder>),
	Filter(Box<dyn Filter>),
	Select(Box<dyn Selector>),
	Grouper(Box<dyn Grouper>),
	Sorter(Box<dyn Sorter>),
	Flatten(bool),
}

impl StageBuilder {
	pub async fn build(self, ctx: &ExecutionContext<'_>, source: Arc<RuleMetadata>) -> Result<Stage, Error> {
		match self {
			StageBuilder::Search(location_builder) => {
				let stage = location_builder.build(ctx).await.unwrap();
				Ok(Stage::Search { location: stage, source })
			}
			StageBuilder::Flatten(bool) => Ok(Stage::Flatten {
				flatten: bool,
				source,
			}),
			StageBuilder::Action(builder) => {
				let stage = builder.build(ctx).await?;
				Ok(Stage::Action { action: stage, source })
			}
			StageBuilder::Filter(stage) => Ok(Stage::Filter { filter: stage, source }),
			StageBuilder::Grouper(stage) => Ok(Stage::Grouper { grouper: stage, source }),
			StageBuilder::Sorter(stage) => Ok(Stage::Sorter { sorter: stage, source }),
			StageBuilder::Compose(_) => unreachable!("Compose stages should be flattened"),
			StageBuilder::Select(stage) => Stage::Select { selector: stage, source },
		}
	}
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Stage {
	Search {
		location: Location,
		source: Arc<RuleMetadata>,
	},
	Action {
		action: Box<dyn Action>,
		source: Arc<RuleMetadata>,
	},
	Filter {
		filter: Box<dyn Filter>,
		source: Arc<RuleMetadata>,
	},
	Select {
		selector: Box<dyn Selector>,
		source: Arc<RuleMetadata>,
	},
	Flatten {
		flatten: bool,
		source: Arc<RuleMetadata>,
	},
	Grouper {
		grouper: Box<dyn Grouper>,
		source: Arc<RuleMetadata>,
	},
	Sorter {
		sorter: Box<dyn Sorter>,
		source: Arc<RuleMetadata>,
	},
}

// impl<'de> Deserialize<'de> for StageBuilder {
// 	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
// 	where
// 		D: Deserializer<'de>,
// 	{
// 		// Deserialize the TOML [[stage]] table into a generic Value.
// 		let mut map: toml::Value = Deserialize::deserialize(deserializer)?;
// 		let table = map
// 			.as_table_mut()
// 			.ok_or_else(|| serde::de::Error::custom("Expected a table for the stage"))?;

// 		// Find the single key that defines the stage type.
// 		let key = {
// 			let keys: Vec<_> = table.keys().cloned().collect();
// 			if keys.len() != 1 {
// 				// This handles the case where a stage has multiple primary keys, like both `filter` and `action`.
// 				// We need to check for this AFTER handling the parameters that live alongside the primary key.
// 				// We will find the primary key first, and then deserialize the rest.
// 			}

// 			let possible_keys = ["search", "compose", "action", "filter", "group-by", "sort-by"];
// 			keys.into_iter().find(|k| possible_keys.contains(&k.as_str())).ok_or_else(|| {
// 				serde::de::Error::custom("Stage must contain one of: 'search', 'compose', 'action', 'filter', 'group-by', or 'sort-by'")
// 			})?
// 		};

// 		// The value associated with the primary key.
// 		let value = table
// 			.remove(&key)
// 			.ok_or_else(|| serde::de::Error::custom(format!("Could not find key '{}'", key)))?;

// 		// The rest of the table contains the parameters.
// 		let params = toml::Value::Table(table.clone());

// 		match key.as_str() {
// 			"search" => {
// 				let path_template = value.try_into::<String>().map_err(serde::de::Error::custom)?;
// 				let mut builder: LocationBuilder = params.try_into().map_err(serde::de::Error::custom)?;
// 				builder.path = Template::from_str(&path_template).map_err(serde::de::Error::custom)?; // Set the path from the primary key's value
// 				Ok(StageBuilder::Search(builder))
// 			}
// 			"compose" => {
// 				let rules_to_compose = value.try_into::<Vec<PathBuf>>().map_err(serde::de::Error::custom)?;
// 				Ok(StageBuilder::Compose(rules_to_compose))
// 			}
// 			"filter" | "action" | "group-by" | "sort-by" => {
// 				// This handles all the typetag'd trait objects.
// 				let component_type = value
// 					.as_str()
// 					.ok_or_else(|| serde::de::Error::custom(format!("Expected a string for key '{}'", key)))?;

// 				// We inject the `type` field that `typetag` expects into the parameters table.
// 				let mut component_table = params.try_into::<toml::value::Table>().map_err(serde::de::Error::custom)?;
// 				component_table.insert("type".to_string(), toml::Value::String(component_type.to_string()));
// 				let component_value = toml::Value::Table(component_table);

// 				// Now deserialize from this new value into the correct trait object.
// 				match key.as_str() {
// 					"filter" => Ok(StageBuilder::Filter(
// 						Box::<dyn Filter>::deserialize(component_value).map_err(serde::de::Error::custom)?,
// 					)),
// 					"action" => Ok(StageBuilder::Action(
// 						Box::<dyn Action>::deserialize(component_value).map_err(serde::de::Error::custom)?,
// 					)),
// 					"group-by" => Ok(StageBuilder::Grouper(
// 						Box::<dyn Grouper>::deserialize(component_value).map_err(serde::de::Error::custom)?,
// 					)),
// 					"sort-by" => Ok(StageBuilder::Sorter(
// 						Box::<dyn Sorter>::deserialize(component_value).map_err(serde::de::Error::custom)?,
// 					)),
// 					_ => unreachable!(),
// 				}
// 			}
// 			other => Err(serde::de::Error::custom(format!("Unknown stage type: '{}'", other))),
// 		}
// 	}
// }

impl<'de> Deserialize<'de> for StageBuilder {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let mut map: toml::Value = Deserialize::deserialize(deserializer)?;
		let table = map
			.as_table_mut()
			.ok_or_else(|| serde::de::Error::custom("Expected a table for the stage"))?;

		let key = {
			let keys: Vec<_> = table.keys().cloned().collect();
			let possible_keys = ["search", "compose", "action", "filter", "group-by", "sort-by"];
			keys.into_iter().find(|k| possible_keys.contains(&k.as_str())).ok_or_else(|| {
				serde::de::Error::custom("Stage must contain one of: 'search', 'compose', 'action', 'filter', 'group-by', or 'sort-by'")
			})?
		};

		let value = table
			.remove(&key)
			.ok_or_else(|| serde::de::Error::custom(format!("Could not find key '{key}'")))?;

		let params = toml::Value::Table(table.clone());

		match key.as_str() {
			"search" => {
				let path_template_str = value.try_into::<String>().map_err(serde::de::Error::custom)?;
				let mut params = params.as_table().unwrap().clone();
				params.insert("path".to_string(), path_template_str.into());
				let builder: LocationBuilder = params.try_into().map_err(serde::de::Error::custom)?;

				Ok(StageBuilder::Search(builder))
			}
			"compose" => {
				let rule_to_compose = value.try_into::<PathBuf>().map_err(serde::de::Error::custom)?;
				Ok(StageBuilder::Compose(rule_to_compose))
			}
			"flatten" => {
				let value = value.try_into::<bool>().map_err(serde::de::Error::custom)?;
				Ok(StageBuilder::Flatten(value))
			}
			"filter" | "select" | "action" | "group-by" | "sort-by" => {
				let component_type = value
					.as_str()
					.ok_or_else(|| serde::de::Error::custom(format!("Expected a string for key '{key}'")))?;

				let mut component_table = params.try_into::<toml::value::Table>().map_err(serde::de::Error::custom)?;
				component_table.insert("type".to_string(), toml::Value::String(component_type.to_string()));
				let component_value = toml::Value::Table(component_table);

				match key.as_str() {
					"filter" => Ok(StageBuilder::Filter(
						Box::<dyn Filter>::deserialize(component_value).map_err(serde::de::Error::custom)?,
					)),
					"select" => Ok(StageBuilder::Select(
						Box::<dyn Selector>::deserialize(component_value).map_err(serde::de::Error::custom)?,
					)),
					"action" => Ok(StageBuilder::Action(
						Box::<dyn ActionBuilder>::deserialize(component_value).map_err(serde::de::Error::custom)?,
					)),
					"group-by" => Ok(StageBuilder::Grouper(
						Box::<dyn Grouper>::deserialize(component_value).map_err(serde::de::Error::custom)?,
					)),
					"sort-by" => Ok(StageBuilder::Sorter(
						Box::<dyn Sorter>::deserialize(component_value).map_err(serde::de::Error::custom)?,
					)),
					_ => unreachable!(),
				}
			}
			other => Err(serde::de::Error::custom(format!("Unknown stage type: '{other}'"))),
		}
	}
}
