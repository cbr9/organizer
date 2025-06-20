use crate::{
	config::{
		actions::{Contract, ExecutionModel},
		context::{Blackboard, ExecutionContext, ExecutionScope, RunServices, RunSettings},
		rule::Rule,
		Config,
		ConfigBuilder,
	},
	resource::Resource,
	templates::Templater,
};
use anyhow::Result;
use futures::{future, stream, StreamExt};
use itertools::Itertools;
use std::{path::PathBuf, sync::Arc};

/// The main engine for the application.
/// It owns the compiled configuration and all run-wide services.
pub struct Engine {
	pub config: Config,
	services: RunServices,
	settings: RunSettings,
}
const CONCURRENT_OPERATIONS: usize = 100;

impl Engine {
	pub fn new(path: &Option<PathBuf>, settings: RunSettings, tags: &Option<Vec<String>>, ids: &Option<Vec<String>>) -> Result<Arc<Self>> {
		let config_builder = ConfigBuilder::new(path.clone())?;
		let mut engine = Templater::from_config(&config_builder)?;
		let config = config_builder.build(&mut engine, tags, ids)?;

		let services = RunServices {
			templater: engine,
			blackboard: Blackboard::default(),
		};
		Ok(Arc::new(Self { config, services, settings }))
	}

	// The signature is a simple `async fn` taking `&self`.
	pub async fn run(self: Arc<Self>) -> Result<()> {
		for rule in self.config.rules.iter() {
			for folder in rule.folders.iter() {
				let resources = match folder.get_resources() {
					Ok(resources) => resources,
					Err(e) => {
						tracing::warn!("Could not get resources from {}: {}", folder.path.display(), e);
						continue;
					}
				};

				// ---- Filtering Stage ----
				// 1. Create a vector of futures without spawning them.
				let engine = Arc::clone(&self);
				let filter_futures = resources
					.into_iter()
					.map(|resource| {
						let engine = engine.clone();
						let rule = rule.clone();
						let folder = folder.clone();

						tokio::spawn(async move {
							let ctx = ExecutionContext {
								services: &engine.services,
								settings: &engine.settings,
								scope: ExecutionScope {
									config: &engine.config,
									rule: &rule,
									folder: &folder,
									resource,
									resources: vec![],
								},
							};
							let mut passed_all_filters = true;
							for filter in &rule.filters {
								if !filter.filter(&ctx).await {
									passed_all_filters = false;
									break;
								}
							}
							if passed_all_filters {
								Some(ctx.scope.resource)
							} else {
								None
							}
						})
					})
					.collect_vec();

				let mut resources: Vec<Resource> = future::join_all(filter_futures)
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
								async move {
									let ctx = ExecutionContext {
										services: &engine.services,
										settings: &engine.settings,
										scope: ExecutionScope {
											config: &engine.config,
											rule: &rule,
											folder: &folder,
											resource,
											resources: vec![],
										},
									};
									// The action is executed, and we await its contract.
									action.execute(&ctx).await.ok()
								}
							});

							let contracts: Vec<Contract> = stream::iter(action_futures)
								.buffer_unordered(CONCURRENT_OPERATIONS)
								.filter_map(|contract| async { contract })
								.collect()
								.await;

							let mut next_resources = vec![];

							for contract in contracts {
								if self.settings.dry_run {
									for resource in &contract.created {
										self.services.blackboard.simulated_paths.insert(resource.path().to_path_buf());
									}
									for resource in &contract.deleted {
										self.services.blackboard.simulated_paths.remove(resource.path());
									}
								}
								next_resources.extend(contract.created);
							}
							next_resources
						}
						ExecutionModel::Batch => {
							todo!()
						}
					};
				}
			}
		}
		Ok(())
	}
}
