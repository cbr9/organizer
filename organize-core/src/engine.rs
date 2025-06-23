use crate::{
	config::{
		actions::{ExecutionModel, Output},
		context::{Blackboard, ExecutionContext, ExecutionScope, FileState, RunServices, RunSettings},
		Config,
		ConfigBuilder,
	},
	journal::Journal,
	path::locker::Locker,
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
	pub async fn new(path: &Option<PathBuf>, settings: RunSettings, tags: &Option<Vec<String>>, ids: &Option<Vec<String>>) -> Result<Arc<Self>> {
		let config_builder = ConfigBuilder::new(path.clone())?;
		let mut engine = Templater::from_config(&config_builder)?;
		let config = config_builder.build(&mut engine, tags, ids)?;

		let journal = Arc::new(Journal::new(&settings).await?);
		let services = RunServices {
			templater: engine,
			blackboard: Blackboard::default(),
			locker: Locker::default(),
			journal,
		};
		Ok(Arc::new(Self { config, services, settings }))
	}

	// The signature is a simple `async fn` taking `&self`.
	pub async fn run(self: Arc<Self>) -> Result<()> {
		let session_id = self.services.journal.start_session(&self.config).await?;
		for rule in self.config.rules.iter() {
			for folder in rule.folders.iter() {
				let engine = Arc::clone(&self);
				let resources = match folder.get_resources() {
					Ok(resources) => resources,
					Err(e) => {
						tracing::warn!("Could not get resources from {}: {}", folder.path.display(), e);
						continue;
					}
				};

				for res in &resources {
					engine
						.services
						.blackboard
						.known_paths
						.entry(res.clone())
						.insert(FileState::Exists);
				}

				// ---- Filtering Stage ----
				// 1. Create a vector of futures without spawning them.
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
									resource: &resource,
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
								Some(resource)
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

								tokio::spawn(async move {
									let ctx = ExecutionContext {
										services: &engine.services,
										settings: &engine.settings,
										scope: ExecutionScope {
											config: &engine.config,
											rule: &rule,
											folder: &folder,
											resource: &resource, // Use the resource for this specific action
										},
									};

									let blackboard = &engine.services.blackboard;
									let receipt = action.commit(&ctx).await?;

									engine
										.services
										.journal
										.record_transaction(session_id, &action, &receipt)
										.await?;

									// Simulate dry run effects (created/deleted paths and backups)
									if engine.settings.dry_run {
										for output in &receipt.outputs {
											match output {
												Output::Created(resource) => {
													blackboard.known_paths.entry(resource.clone()).insert(FileState::Exists);
												}
												Output::Deleted(resource) => {
													blackboard.known_paths.entry(resource.clone()).insert(FileState::Deleted);
												}
												Output::Modified(_resource) => {}
											};
										}
										for undo_op in receipt.undo.iter() {
											if let Some(backup_path) = undo_op.backup() {
												blackboard.known_paths.entry(backup_path.0.clone()).insert(FileState::Exists);
											}
										}
									}

									Ok(receipt.next)
								})
							});

							stream::iter(action_futures)
								.buffer_unordered(CONCURRENT_OPERATIONS)
								.filter_map(|join_handle_result| async move {
									join_handle_result
										.ok()
										.and_then(|action_execution_result: Result<Vec<Resource>>| action_execution_result.ok())
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
