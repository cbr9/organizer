pub mod batch;
pub mod pipeline;
pub mod rule;
pub mod stage;

use crate::{
	context::{
		services::{fs::manager::FileSystemManager, history::Journal, Blackboard, RunServices},
		ExecutionContext,
		scope::ExecutionScope,
		settings::RunSettings,
	},
	engine::{
		pipeline::Pipeline,
		rule::{Rule, RuleBuilder},
	},
	templates::compiler::TemplateCompiler,
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
	services: RunServices,
	settings: RunSettings,
}

impl Engine {
	pub async fn new(path: &PathBuf, settings: RunSettings) -> Result<Arc<Self>> {
		let content = tokio::fs::read_to_string(path).await?;
		let builder: RuleBuilder = toml::from_str(&content)?;
		let services = RunServices {
			blackboard: Blackboard::default(),
			journal: Arc::new(Journal::new(&settings).await?),
			fs: FileSystemManager::new(&builder),
			compiler: TemplateCompiler::new(),
		};
		let rule = {
			let ctx = ExecutionContext {
				services: &services,
				scope: ExecutionScope::Blank,
				settings: &settings,
			};
			let rule = builder.build(&ctx).await?;
			rule
		};

		Ok(Arc::new(Self { rule, services, settings }))
	}

	pub async fn run(&self) -> Result<()> {
		let pipeline = Pipeline::new(self.rule.clone());

		// Create the top-level execution context with a blank scope.
		let ctx = ExecutionContext {
			services: &self.services,
			settings: &self.settings, // Assuming you have settings
			scope: ExecutionScope::Blank,
		};

		let _final_stream = pipeline.run(&ctx).await?;
		Ok(())
	}
}