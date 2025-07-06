use std::{path::PathBuf, sync::Arc};

use serde::{Deserialize, Deserializer, Serialize};

use crate::{
	context::ExecutionContext,
	engine::rule::RuleMetadata,
	error::Error,
	location::{Location, LocationBuilder},
	plugins::{
		action::{Action, ActionBuilder},
		filter::Filter,
		partitioner::Partitioner,
		selector::Selector,
		sorter::Sorter,
	},
};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct StageParams {
	#[serde(default)]
	pub description: Option<String>,
	#[serde(default = "default_true")]
	pub enabled: bool,
	#[serde(default)]
	pub on_batches: Option<Vec<String>>,
	#[serde(default)]
	pub check: Option<PathBuf>,
}

fn default_true() -> bool {
	true
}

#[derive(Debug, Serialize, PartialEq, Eq, Clone)]
pub enum StageBuilder {
	Search(LocationBuilder, StageParams),
	Compose(PathBuf),
	Action(Box<dyn ActionBuilder>, StageParams),
	Filter(Box<dyn Filter>, StageParams),
	Select(Box<dyn Selector>, StageParams),
	Partition(Box<dyn Partitioner>, StageParams),
	Sort(Box<dyn Sorter>, StageParams),
	Flatten(bool, StageParams),
}

impl StageBuilder {
	pub async fn build(self, ctx: &ExecutionContext, source: Arc<RuleMetadata>) -> Result<Stage, Error> {
		match self {
			StageBuilder::Search(location_builder, params) => {
				let stage = location_builder.build(ctx).await.unwrap();
				Ok(Stage::Search {
					location: Arc::new(stage),
					params,
					source,
				})
			}
			StageBuilder::Flatten(flatten, params) => Ok(Stage::Flatten { flatten, params, source }),
			StageBuilder::Action(builder, params) => {
				let stage = builder.build(ctx).await?;
				Ok(Stage::Action {
					action: stage,
					params,
					source,
				})
			}
			StageBuilder::Filter(filter, params) => Ok(Stage::Filter { filter, params, source }),
			StageBuilder::Partition(partitioner, params) => Ok(Stage::Partition { partitioner, params, source }),
			StageBuilder::Sort(sorter, params) => Ok(Stage::Sort { sorter, params, source }),
			StageBuilder::Compose(_) => unreachable!("Compose stages should be flattened"),
			StageBuilder::Select(selector, params) => Ok(Stage::Select { selector, params, source }),
		}
	}
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Stage {
	Search {
		location: Arc<Location>,
		params: StageParams,
		source: Arc<RuleMetadata>,
	},
	Action {
		action: Box<dyn Action>,
		params: StageParams,
		source: Arc<RuleMetadata>,
	},
	Filter {
		filter: Box<dyn Filter>,
		params: StageParams,
		source: Arc<RuleMetadata>,
	},
	Select {
		selector: Box<dyn Selector>,
		params: StageParams,
		source: Arc<RuleMetadata>,
	},
	Flatten {
		flatten: bool,
		params: StageParams,
		source: Arc<RuleMetadata>,
	},
	Partition {
		partitioner: Box<dyn Partitioner>,
		params: StageParams,
		source: Arc<RuleMetadata>,
	},
	Sort {
		sorter: Box<dyn Sorter>,
		params: StageParams,
		source: Arc<RuleMetadata>,
	},
}

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
			let possible_keys = ["search", "compose", "action", "filter", "partition-by", "sort-by", "select", "flatten"];
			keys.into_iter().find(|k| possible_keys.contains(&k.as_str())).ok_or_else(|| {
				serde::de::Error::custom(
					"Stage must contain one of: 'search', 'compose', 'action', 'filter', 'partition-by', 'sort-by', 'select', 'flatten'",
				)
			})?
		};

		let value = table
			.remove(&key)
			.ok_or_else(|| serde::de::Error::custom(format!("Could not find key '{key}'")))?;

		let params: StageParams = toml::Value::Table(table.clone()).try_into().map_err(serde::de::Error::custom)?;
		table.remove("on_batches");
		table.remove("enabled");
		table.remove("description");
		table.remove("check");

		match key.as_str() {
			"search" => {
				let path_template_str = value.try_into::<String>().map_err(serde::de::Error::custom)?;
				let mut params_table = table.clone();
				params_table.insert("path".to_string(), path_template_str.into());
				let builder: LocationBuilder = toml::Value::Table(params_table).try_into().map_err(serde::de::Error::custom)?;

				Ok(StageBuilder::Search(builder, params))
			}
			"compose" => {
				let rule_to_compose = value.try_into::<PathBuf>().map_err(serde::de::Error::custom)?;
				Ok(StageBuilder::Compose(rule_to_compose))
			}
			"flatten" => {
				let value = value.try_into::<bool>().map_err(serde::de::Error::custom)?;
				Ok(StageBuilder::Flatten(value, params))
			}
			"filter" | "select" | "action" | "partition-by" | "sort-by" => {
				let component_type = value
					.as_str()
					.ok_or_else(|| serde::de::Error::custom(format!("Expected a string for key '{key}'")))?;

				let mut component_table = table.clone();
				component_table.insert("type".to_string(), toml::Value::String(component_type.to_string()));
				let component_value = toml::Value::Table(component_table);

				match key.as_str() {
					"filter" => Ok(StageBuilder::Filter(
						Box::<dyn Filter>::deserialize(component_value).map_err(serde::de::Error::custom)?,
						params,
					)),
					"partition-by" => Ok(StageBuilder::Partition(
						Box::<dyn Partitioner>::deserialize(component_value).map_err(serde::de::Error::custom)?,
						params,
					)),
					"select" => Ok(StageBuilder::Select(
						Box::<dyn Selector>::deserialize(component_value).map_err(serde::de::Error::custom)?,
						params,
					)),
					"action" => Ok(StageBuilder::Action(
						Box::<dyn ActionBuilder>::deserialize(component_value).map_err(serde::de::Error::custom)?,
						params,
					)),
					"sort-by" => Ok(StageBuilder::Sort(
						Box::<dyn Sorter>::deserialize(component_value).map_err(serde::de::Error::custom)?,
						params,
					)),
					_ => unreachable!(),
				}
			}
			other => Err(serde::de::Error::custom(format!("Unknown stage type: '{other}'"))),
		}
	}
}
