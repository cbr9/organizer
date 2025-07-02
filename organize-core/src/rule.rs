use std::{path::PathBuf, sync::Arc};

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
	splitter::Splitter,
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
	Group(Box<dyn Grouper>),
	Split(Box<dyn Splitter>),
	Sort(Box<dyn Sorter>),
	Flatten(bool),
}

impl StageBuilder {
	pub async fn build(self, ctx: &ExecutionContext<'_>, source: Arc<RuleMetadata>) -> Result<Stage, Error> {
		match self {
			StageBuilder::Search(location_builder) => {
				let stage = location_builder.build(ctx).await.unwrap();
				Ok(Stage::Search { location: stage, source })
			}
			StageBuilder::Flatten(bool) => Ok(Stage::Flatten { flatten: bool, source }),
			StageBuilder::Action(builder) => {
				let stage = builder.build(ctx).await?;
				Ok(Stage::Action { action: stage, source })
			}
			StageBuilder::Filter(filter) => Ok(Stage::Filter { filter, source }),
			StageBuilder::Group(grouper) => Ok(Stage::Group { grouper, source }),
			StageBuilder::Sort(sorter) => Ok(Stage::Sort { sorter, source }),
			StageBuilder::Compose(_) => unreachable!("Compose stages should be flattened"),
			StageBuilder::Select(selector) => Ok(Stage::Select { selector, source }),
			StageBuilder::Split(splitter) => Ok(Stage::Split { splitter, source }),
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
	Group {
		grouper: Box<dyn Grouper>,
		source: Arc<RuleMetadata>,
	},
	Split {
		splitter: Box<dyn Splitter>,
		source: Arc<RuleMetadata>,
	},
	Sort {
		sorter: Box<dyn Sorter>,
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
			let possible_keys = ["search", "compose", "action", "filter", "group-by", "sort-by", "split-by", "select"];
			keys.into_iter().find(|k| possible_keys.contains(&k.as_str())).ok_or_else(|| {
				serde::de::Error::custom("Stage must contain one of: 'search', 'compose', 'action', 'filter', 'group-by', 'sort-by', 'split-by', 'select'")
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
			"filter" | "select" | "action" | "split-by" | "group-by" | "sort-by" => {
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
					"split-by" => Ok(StageBuilder::Split(
						Box::<dyn Splitter>::deserialize(component_value).map_err(serde::de::Error::custom)?,
					)),
					"select" => Ok(StageBuilder::Select(
						Box::<dyn Selector>::deserialize(component_value).map_err(serde::de::Error::custom)?,
					)),
					"action" => Ok(StageBuilder::Action(
						Box::<dyn ActionBuilder>::deserialize(component_value).map_err(serde::de::Error::custom)?,
					)),
					"group-by" => Ok(StageBuilder::Group(
						Box::<dyn Grouper>::deserialize(component_value).map_err(serde::de::Error::custom)?,
					)),
					"sort-by" => Ok(StageBuilder::Sort(
						Box::<dyn Sorter>::deserialize(component_value).map_err(serde::de::Error::custom)?,
					)),
					_ => unreachable!(),
				}
			}
			other => Err(serde::de::Error::custom(format!("Unknown stage type: '{other}'"))),
		}
	}
}
