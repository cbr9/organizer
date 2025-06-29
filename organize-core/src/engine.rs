use crate::{
	action::{Action, Output, Receipt},
	batch::Batch,
	config::Config,
	context::{
		services::{fs::manager::FileSystemManager, history::Journal},
		Blackboard,
		ExecutionContext,
		ExecutionScope,
		RunServices,
		RunSettings,
	},
	errors::Error,
	filter::Filter,
	options::Options,
	resource::Resource,
	rule::{Rule, Stage},
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

impl Engine {
	pub async fn new(path: &Option<PathBuf>, settings: RunSettings, tags: &Option<Vec<String>>, ids: &Option<Vec<String>>) -> Result<Arc<Self>> {
		let config = Config::new(path.clone(), tags, ids)?;

		let services = RunServices {
			blackboard: Blackboard::default(),
			journal: Arc::new(Journal::new(&settings).await?),
			fs: FileSystemManager::default(),
		};
		Ok(Arc::new(Self { config, services, settings }))
	}

	pub async fn run(&self) -> Result<()> {
		for rule in &self.config.rules {
			self.process_rule(rule).await?;
		}
		Ok(())
	}

	async fn process_rule(&self, rule: &Rule) -> Result<()> {
		for location in &rule.locations {
			let ctx = &ExecutionContext {
				services: &self.services,
				scope: ExecutionScope::new_location_scope(&self.config, rule, location),
				settings: &self.settings,
			};
			let path = location.partial_path().render(ctx).await.map(PathBuf::from)?;
			location.initialize_path(path);
			let final_options = Options::compile(&self.config.defaults, &rule.options, location.partial_options());
			location.initialize_options(final_options);
		}
		let ctx = &ExecutionContext {
			services: &self.services,
			scope: ExecutionScope::new_rule_scope(&self.config, rule),
			settings: &self.settings,
		};
		let mut initial_files = Vec::new();
		for location in &rule.locations {
			let mut paths_in_folder = location.get_resources(ctx).await?;
			initial_files.append(&mut paths_in_folder);
		}

		// 3. INITIALIZE & RUN PIPELINE: Start the pipeline with a single batch.
		let initial_batch = Batch::initial(initial_files);
		self.process_pipeline(&rule.pipeline, vec![initial_batch], rule).await?;

		Ok(())
	}

	async fn execute_filter_stage(&self, filter: &Box<dyn Filter>, batch: &Batch, rule: &Rule) -> Result<Vec<Arc<Resource>>> {
		let passed = match filter.execution_model() {
			ExecutionModel::Batch => {
				let scope = ExecutionScope::new_batch_scope(&self.config, rule, &batch);
				let ctx = ExecutionContext {
					services: &self.services,
					scope,
					settings: &self.settings,
				};
				filter.filter(&ctx).await?
			}
			ExecutionModel::Single => {
				let mut futs = Vec::new();
				for resource in &batch.files {
					let resource_clone = resource.clone();
					// We create a new future for each file using an `async move` block.
					let fut = async move {
						// All the data needed is moved into this block.
						// The scope and context are now created and live only inside this future.
						let scope = ExecutionScope::new_resource_scope(&self.config, rule, resource_clone);
						let ctx = ExecutionContext {
							services: &self.services,
							scope,
							settings: &self.settings,
						};
						// We await the filter's result *inside* the block.
						filter.filter(&ctx).await
					};
					futs.push(fut);
				}

				// `join_all` now runs our self-contained futures.
				let results: Vec<Result<Vec<Arc<Resource>>, Error>> = future::join_all(futs).await;

				// The rest of the logic for collecting results remains the same.
				let passed_files = results
					.into_iter()
					.filter_map(|res| match res {
						Ok(filter_result) => Some(filter_result),
						Err(e) => {
							eprintln!("Filter error on a file, skipping it: {}", e);
							None
						}
					})
					.flatten()
					.collect();
				passed_files
			}
		};
		Ok(passed)
	}

	/// Executes an action stage according to its execution model.
	async fn execute_action_stage(&self, action: &Box<dyn Action>, batch: &Batch, rule: &Rule) -> Result<Receipt, Error> {
		match action.execution_model() {
			ExecutionModel::Batch => {
				let scope = ExecutionScope::new_batch_scope(&self.config, rule, &batch);
				let ctx = ExecutionContext {
					services: &self.services,
					scope,
					settings: &self.settings,
				};
				action.commit(&ctx).await
			}
			ExecutionModel::Single => {
				let mut futs = Vec::new();
				for resource in &batch.files {
					// Clone the Arc for the resource and any other needed data.
					let resource_clone = resource.clone();

					// Create a self-contained future with an `async move` block.
					let fut = async move {
						// All context is now created and owned within the future.
						let scope = ExecutionScope::new_resource_scope(&self.config, rule, resource_clone);
						let ctx = ExecutionContext {
							services: &self.services,
							scope,
							settings: &self.settings,
						};
						action.commit(&ctx).await
					};
					futs.push(fut);
				}

				let results: Vec<Result<Receipt, Error>> = future::join_all(futs).await;
				let mut combined_receipt = Receipt::default();

				for result in results {
					match result {
						Ok(receipt) => {
							combined_receipt.outputs.extend(receipt.outputs);
							combined_receipt.inputs.extend(receipt.inputs);
							combined_receipt.next.extend(receipt.next);
							combined_receipt.undo.extend(receipt.undo);
						}
						Err(e) => {
							// If any single action fails, we fail the entire batch.
							// This is a safe default for actions.
							return Err(e);
						}
					}
				}
				Ok(combined_receipt)
			}
		}
	}

	async fn process_pipeline(&self, pipeline: &[Stage], mut batches: Vec<Batch>, rule: &Rule) -> Result<()> {
		if pipeline.is_empty() || batches.is_empty() {
			return Ok(());
		}

		let (stage, next_pipeline) = pipeline.split_at(1);
		let stage = &stage[0];
		let mut next_batches = Vec::new();

		match stage {
			Stage::Filter(filter) => {
				for batch in batches {
					if let Ok(result) = self.execute_filter_stage(filter, &batch, rule).await {
						if !result.is_empty() {
							next_batches.push(Batch {
								files: result,
								context: batch.context,
							});
						}
					}
				}
			}
			Stage::Action(action) => {
				for batch in batches {
					if let Ok(result) = self.execute_action_stage(action, &batch, rule).await {
						if !result.next.is_empty() {
							// The action's result forms a new batch, but context is lost unless
							// we explicitly design the action to pass it through.
							next_batches.push(Batch::initial(result.next));
						}
					}
				}
			}
			Stage::Grouper(grouper) => {
				for batch in batches {
					let mut grouped_sub_batches = grouper.group(&batch).await;
					next_batches.append(&mut grouped_sub_batches);
				}
			}
			Stage::Sorter(sorter) => {
				for batch in &mut batches {
					sorter.sort(&mut batch.files).await;
				}
				// The `next_batches` are the same batches, just with their internal file lists sorted.
				next_batches = batches;
			}
		}

		Box::pin(self.process_pipeline(next_pipeline, next_batches, rule)).await
	}
}
