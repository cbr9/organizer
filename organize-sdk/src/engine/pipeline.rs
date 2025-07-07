use futures::future;

use crate::{
	context::{scope::ExecutionScope, services::fs::resource::Resource, ExecutionContext},
	engine::{
		batch::Batch,
		rule::Rule,
		stage::{Stage, StageParams},
		ExecutionModel,
	},
	error::Error,
	plugins::{action::Receipt, partitioner::Partitioner, sorter::Sorter},
};
use glob::Pattern;
use std::{
	collections::{HashMap, HashSet},
	sync::Arc,
};

/// Represents the data flowing through the pipeline.
/// It tracks the current set of file batches and the sequence of
/// partitioners that have been applied to create them.
#[derive(Debug)]
pub struct PipelineStream {
	/// The current data, always represented as a list of batches.
	/// An "ungrouped" state is simply a Vec with one Batch.
	pub batches: HashMap<String, Batch>,
	/// The ordered stack of partitioners that have been applied.
	pub partitioners: Vec<Box<dyn Partitioner>>,
	pub sorters: Vec<Box<dyn Sorter>>,
}

impl PipelineStream {
	/// Creates a new stream with a single batch of files and no groupings.
	pub fn new(files: Vec<Arc<Resource>>) -> Self {
		Self {
			batches: HashMap::from([("root".into(), Batch::initial(files))]),
			partitioners: Vec::new(),
			sorters: Vec::new(),
		}
	}

	/// Flattens all batches into a single, unordered list of files.
	pub fn all_files(&self) -> Vec<Arc<Resource>> {
		self.batches.values().flat_map(|batch| batch.files.clone()).collect()
	}

	pub async fn resort(&mut self) {
		for batch in self.batches.values_mut() {
			for sorter in &self.sorters {
				sorter.sort(&mut batch.files).await;
			}
		}
	}

	/// Re-applies the entire stack of stored partitioners to a new set of files.
	/// This is the key to maintaining a consistent state.
	pub async fn repartition(&self, files: Vec<Arc<Resource>>) -> Result<HashMap<String, Batch>, anyhow::Error> {
		let mut current_batches = HashMap::from([("root".into(), Batch::initial(files))]);

		for partitioner in &self.partitioners {
			let mut next_level_batches = HashMap::new();
			for (parent_name, parent_batch) in &current_batches {
				let named_batches_map = partitioner.partition(parent_batch).await?;
				for (new_key_part, mut sub_batch) in named_batches_map {
					let new_name = if parent_name == "root" {
						new_key_part.clone()
					} else {
						format!("{parent_name}.{new_key_part}")
					};
					sub_batch.context.extend(parent_batch.context.clone());
					sub_batch.context.insert(partitioner.name().to_string(), new_key_part.clone());
					next_level_batches.insert(new_name, sub_batch);
				}
			}
			current_batches = next_level_batches;
		}
		Ok(current_batches)
	}
}

pub struct Pipeline {
	stages: Vec<Stage>,
	stream: PipelineStream,
}

fn select_batches<'a>(
	all_batches: &'a HashMap<String, Batch>,
	params: &StageParams,
) -> (HashMap<String, &'a Batch>, HashMap<String, &'a Batch>, Vec<String>) {
	let mut selected_batches = HashMap::new();
	let mut unselected_batches = HashMap::new();
	let mut unmatched_patterns = Vec::new();

	if let Some(patterns) = &params.on_batches {
		let all_batch_names: HashSet<_> = all_batches.keys().cloned().collect();
		let mut matched_names = HashSet::new();

		for pattern_str in patterns {
			let pattern = Pattern::new(pattern_str).unwrap();
			let mut matched_any = false;
			for name in &all_batch_names {
				if pattern.matches(name) {
					matched_names.insert(name.clone());
					matched_any = true;
				}
			}
			if !matched_any {
				unmatched_patterns.push(pattern_str.clone());
			}
		}

		for (name, batch) in all_batches {
			if matched_names.contains(name) {
				selected_batches.insert(name.clone(), batch);
			} else {
				unselected_batches.insert(name.clone(), batch);
			}
		}
	} else {
		// If no patterns are specified, select all batches
		for (name, batch) in all_batches {
			selected_batches.insert(name.clone(), batch);
		}
	}

	(selected_batches, unselected_batches, unmatched_patterns)
}

impl Stage {
	fn params(&self) -> Option<&StageParams> {
		match self {
			Stage::Action { params, .. } => Some(params),
			Stage::Filter { params, .. } => Some(params),
			Stage::Partition { params, .. } => Some(params),
			Stage::Sort { params, .. } => Some(params),
			Stage::Select { params, .. } => Some(params),
			Stage::Search { params, .. } => Some(params),
			Stage::Flatten { params, .. } => Some(params),
		}
	}
}

impl Pipeline {
	pub fn new(rule: Rule) -> Self {
		Self {
			stages: rule.pipeline,
			stream: PipelineStream::new(Vec::new()), // Start with no files
		}
	}

	pub async fn run(mut self, ctx: &ExecutionContext) -> Result<PipelineStream, Error> {
		if ctx.settings.dry_run {
			ctx.services.reporter.ui.warning(
				"This is a simulation. No real I/O operations will be performed. If you want to apply the reported changes, rerun the application \
				 with the --no-dry-run flag.",
			);
		}
		for stage in self.stages.into_iter() {
			if let Some(params) = stage.params() {
				if !params.enabled {
					continue;
				}
				if let Some(description) = &params.description {
					tracing::debug!(description = %description, "Running stage");
				}
			}
			match stage {
				Stage::Search { location, source, .. } => {
					let scope = ExecutionScope::new_location_scope(source.clone(), location.clone());
					let ctx = ctx.with_scope(scope);
					let new_files = ctx.services.fs.get_provider(&location.host)?.discover(&location, &ctx).await?;
					if location.mode.is_replace() {
						if location.keep_structure {
							self.stream.batches = self.stream.repartition(new_files).await?;
							self.stream.resort().await;
						} else {
							self.stream = PipelineStream::new(new_files);
						}
					} else {
						let mut all_files = self.stream.all_files();
						all_files.extend(new_files);
						self.stream.batches = self.stream.repartition(all_files).await?;
						self.stream.resort().await;
					}
				}
				Stage::Partition { partitioner, params, .. } => {
					let (selected_batches, unselected, unmatched) = select_batches(&self.stream.batches, &params);
					if !unmatched.is_empty() {
						println!(
							"Warning: The following patterns in `on_batches` did not match any existing batches: {}",
							unmatched.join(", ")
						);
					}

					let mut next_level_batches: HashMap<String, Batch> = unselected.into_iter().map(|(k, v)| (k, v.clone())).collect();

					for (parent_name, parent_batch) in selected_batches {
						let named_batches_map = partitioner.partition(parent_batch).await?;
						for (new_key_part, mut sub_batch) in named_batches_map {
							let new_name = if parent_name == "root" {
								new_key_part.clone()
							} else {
								format!("{parent_name}.{new_key_part}")
							};
							sub_batch.context.extend(parent_batch.context.clone());
							sub_batch.context.insert(partitioner.name().to_string(), new_key_part);
							next_level_batches.insert(new_name, sub_batch);
						}
					}
					self.stream.batches = next_level_batches;
					self.stream.partitioners.push(partitioner);
					self.stream.resort().await;
				}
				Stage::Sort { sorter, params, .. } => {
					if params.on_batches.is_some() {
						let (selected_batches, _, unmatched) = select_batches(&self.stream.batches, &params);
						if !unmatched.is_empty() {
							println!(
								"Warning: The following patterns in `on_batches` did not match any existing batches: {}",
								unmatched.join(", ")
							);
						}
						let selected_names: Vec<String> = selected_batches.keys().cloned().collect();
						for name in selected_names {
							if let Some(batch) = self.stream.batches.get_mut(&name) {
								sorter.sort(&mut batch.files).await;
							}
						}
					} else {
						self.stream.sorters.push(sorter);
						self.stream.resort().await;
					}
				}
				Stage::Filter { filter, params, source } => {
					let check_path = params.check.as_ref();
					let (selected_batches, unselected, unmatched) = select_batches(&self.stream.batches, &params);
					if !unmatched.is_empty() {
						println!(
							"Warning: The following patterns in `on_batches` did not match any existing batches: {}",
							unmatched.join(", ")
						);
					}

					let mut next_batches: HashMap<String, Batch> = unselected.into_iter().map(|(k, v)| (k, v.clone())).collect();

					match filter.execution_model() {
						ExecutionModel::Batch => {
							for (name, batch) in selected_batches {
								let scope = ExecutionScope::new_batch_scope(source.clone(), Arc::new(batch.clone()));
								let batch_ctx = ctx.with_scope(scope);
								let passed_files = filter.filter(check_path, &batch_ctx).await?;
								if !passed_files.is_empty() {
									next_batches.insert(name.clone(), Batch {
										files: passed_files,
										context: batch.context.clone(),
									});
								}
							}
						}
						ExecutionModel::Single => {
							for (name, batch) in selected_batches {
								let mut futs = Vec::new();
								for resource in &batch.files {
									let resource_clone = resource.clone();
									let meta = source.clone();
									let filter = filter.clone();
									let fut = async move {
										let scope = ExecutionScope::new_resource_scope(meta.clone(), resource_clone);
										let ctx = ctx.with_scope(scope);
										filter.filter(check_path, &ctx).await
									};
									futs.push(fut);
								}
								let results: Vec<Arc<Resource>> = future::try_join_all(futs).await?.into_iter().flatten().collect();
								if !results.is_empty() {
									next_batches.insert(name.clone(), Batch {
										files: results,
										context: batch.context.clone(),
									});
								}
							}
						}
					}
					self.stream.batches = next_batches;
				}
				Stage::Action { action, params, source } => {
					let (selected_batches, unselected, unmatched) = select_batches(&self.stream.batches, &params);
					if !unmatched.is_empty() {
						println!(
							"Warning: The following patterns in `on_batches` did not match any existing batches: {}",
							unmatched.join(", ")
						);
					}

					let mut next_stream_batches: HashMap<String, Batch> = unselected.into_iter().map(|(k, v)| (k, v.clone())).collect();

					for (name, batch) in selected_batches {
						let mut current_batch_next_files = Vec::new();
						match action.execution_model() {
							ExecutionModel::Batch => {
								let scope = ExecutionScope::new_batch_scope(source.clone(), Arc::new(batch.clone()));
								let batch_ctx = ctx.with_scope(scope);
								let ctx = Arc::new(batch_ctx);
								let receipt = action.commit(ctx.clone()).await?;
								current_batch_next_files.extend(receipt.next);
							}
							ExecutionModel::Single => {
								let mut futs = Vec::new();
								for resource in &batch.files {
									let resource_clone = resource.clone();
									let meta = source.clone();
									let action = action.clone();
									let fut = async move {
										let scope = ExecutionScope::new_resource_scope(meta.clone(), resource_clone);
										let ctx = Arc::new(ctx.with_scope(scope));
										action.commit(ctx).await
									};
									futs.push(fut);
								}
								let receipts: Vec<Receipt> = future::try_join_all(futs).await?;
								for receipt in receipts {
									current_batch_next_files.extend(receipt.next);
								}
							}
						}

						if !current_batch_next_files.is_empty() {
							next_stream_batches.insert(name.clone(), Batch {
								files: current_batch_next_files,
								context: batch.context.clone(),
							});
						}
					}
					self.stream.batches = next_stream_batches;
				}
				Stage::Flatten { flatten, .. } => {
					if flatten {
						self.stream = PipelineStream::new(self.stream.all_files());
					}
				}
				Stage::Select { selector, params, .. } => {
					let (selected_batches, unselected, unmatched) = select_batches(&self.stream.batches, &params);
					if !unmatched.is_empty() {
						println!(
							"Warning: The following patterns in `on_batches` did not match any existing batches: {}",
							unmatched.join(", ")
						);
					}

					let mut next_batches: HashMap<String, Batch> = unselected.into_iter().map(|(k, v)| (k, v.clone())).collect();

					for (name, batch) in selected_batches {
						let selected_batch = selector.select(batch).await?;
						if !selected_batch.files.is_empty() {
							next_batches.insert(name.clone(), selected_batch);
						}
					}
					self.stream.batches = next_batches;
				}
			}
		}
		Ok(self.stream)
	}
}
