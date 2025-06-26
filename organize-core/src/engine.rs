use crate::{
	config::{Config, ConfigBuilder},
	context::{
		services::{fs::manager::FileSystemManager, history::Journal},
		Blackboard,
		ExecutionContext,
		ExecutionScope,
		RunServices,
		RunSettings,
	},
	resource::Resource,
	templates::engine::Templater,
};
use anyhow::Result;
use futures::{future, stream, StreamExt};
use itertools::Itertools;
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
	pub config: Config,
	services: RunServices,
	settings: RunSettings,
}
const CONCURRENT_OPERATIONS: usize = 100;

impl Engine {
	pub async fn new(path: &Option<PathBuf>, settings: RunSettings, tags: &Option<Vec<String>>, ids: &Option<Vec<String>>) -> Result<Arc<Self>> {
		let config = ConfigBuilder::new(path.clone())?.build(tags, ids)?;
		let engine = Templater::from_config(&config);

		let services = RunServices {
			templater: engine,
			blackboard: Blackboard::default(),
			journal: Arc::new(Journal::new(&settings).await?),
			fs: FileSystemManager::default(),
		};
		Ok(Arc::new(Self { config, services, settings }))
	}

	// The signature is a simple `async fn` taking `&self`.
	pub async fn run(self: Arc<Self>) -> Result<()> {
		let session_id = self.services.journal.start_session(&self.config).await?;
		for rule in self.config.rules.iter() {
			for folder in rule.folders.iter() {
				let engine = Arc::clone(&self);
				let resources = match folder.get_resources(&self.services).await {
					Ok(resources) => resources.into_iter().map(Arc::new).collect_vec(),
					Err(e) => {
						tracing::warn!("Could not get resources from {}: {}", folder.path.display(), e);
						continue;
					}
				};

				// ---- Filtering Stage ----
				// 1. Create a vector of futures without spawning them.
				let filter_futures = resources
					.into_iter()
					.map(|resource| {
						let engine = engine.clone();
						let rule = rule.clone();
						let folder = folder.clone();

						tokio::spawn(async move {
							let resource = resource.clone();
							let ctx = ExecutionContext {
								services: &engine.services,
								settings: &engine.settings,
								scope: ExecutionScope::new_resource_scope(&engine.config, &rule, &folder, resource.clone()),
							};
							let mut passed_all_filters = true;
							for filter in &rule.filters {
								if !filter.filter(&ctx).await {
									passed_all_filters = false;
									break;
								}
							}
							if passed_all_filters {
								Some(resource)
							} else {
								None
							}
						})
					})
					.collect_vec();

				let mut resources: Vec<Arc<Resource>> = future::join_all(filter_futures)
					.await
					.into_iter()
					.filter_map(|a| a.ok())
					.flatten()
					.collect();

				// ---- Action Stage ----
				for action in rule.actions.iter() {
					resources = match action.execution_model() {
						ExecutionModel::Single => {
							// 1. Create a vector of action futures.
							let action_futures = resources.into_iter().map(|resource| {
								let engine = self.clone();
								let action = action.clone();
								let rule = rule.clone();
								let folder = folder.clone();

								tokio::spawn(async move {
									let ctx = ExecutionContext {
										services: &engine.services,
										settings: &engine.settings,
										scope: ExecutionScope::new_resource_scope(&engine.config, &rule, &folder, resource),
									};

									let receipt = action.commit(&ctx).await?;

									engine
										.services
										.journal
										.record_transaction(session_id, &action, &receipt)
										.await?;

									Ok(receipt.next)
								})
							});

							stream::iter(action_futures)
								.buffer_unordered(CONCURRENT_OPERATIONS)
								.filter_map(|join_handle_result| async move {
									join_handle_result
										.ok()
										.and_then(|action_execution_result: Result<Vec<Arc<Resource>>>| action_execution_result.ok())
								})
								.fold(Vec::new(), |mut acc, resources_vec| async move {
									// For each Vec<Resource> that comes from the stream, extend the accumulator.
									acc.extend(resources_vec);
									acc
								})
								.await
						}
						ExecutionModel::Batch => {
							todo!()
						}
					};
				}
			}
		}
		self.services.journal.end_session(session_id, "success").await?;
		Ok(())
	}
}
