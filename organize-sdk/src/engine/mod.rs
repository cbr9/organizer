pub mod batch;
pub mod pipeline;
pub mod rule;
pub mod stage;

use crate::{
	context::{
		scope::ExecutionScope,
		services::{fs::connections::Connections, reporter::ui::UserInterface, RunServices},
		settings::RunSettings,
		ExecutionContext,
	},
	engine::{
		pipeline::Pipeline,
		rule::{Rule, RuleBuilder},
	},
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
use strum::Display;

#[derive(Default)]
pub enum ExecutionModel {
	#[default]
	Single,
	Batch,
}

#[derive(Eq, Display, PartialEq, Default, Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
#[strum(serialize_all = "snake_case")]
pub enum ConflictResolution {
	Overwrite,
	#[default]
	Skip,
	Rename,
}

/// The main engine for the application.
/// It owns the compiled configuration and all run-wide services.
pub struct Engine {
	rule: Rule,
	services: Arc<RunServices>,
	settings: Arc<RunSettings>,
}

impl Engine {
	pub async fn new(path: &PathBuf, ui: Arc<dyn UserInterface>, settings: RunSettings) -> Result<Arc<Self>> {
		let connections = Connections::from_config_dir().await?;
		let ExecutionContext { services, scope, settings } = ExecutionContext::new(settings, connections, ui).await?;

		let content = tokio::fs::read_to_string(path).await?;
		let builder: RuleBuilder = toml::from_str(&content)?;
		let ctx = ExecutionContext {
			services: services.clone(),
			scope: scope.clone(),
			settings: settings.clone(),
		};
		let rule = builder.build(&ctx).await?;

		Ok(Arc::new(Self { rule, services, settings }))
	}

	pub async fn run(&self) -> Result<()> {
		let pipeline = Pipeline::new(self.rule.clone());

		// Create the top-level execution context with a blank scope.
		let ctx = ExecutionContext {
			services: self.services.clone(),
			settings: self.settings.clone(), // Assuming you have settings
			scope: ExecutionScope::Blank,
		};

		let _final_stream = pipeline.run(&ctx).await?;
		Ok(())
	}
}
