use std::{path::PathBuf, sync::Arc};

use serde::{Deserialize, Deserializer, Serialize};

use crate::{
	context::ExecutionContext,
	engine::stage::{Stage, StageBuilder, StageParams},
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
