use futures::future;

use crate::{
	action::Receipt,
	batch::Batch,
	context::{ExecutionContext, ExecutionScope},
	engine::ExecutionModel,
	errors::Error,
	grouper::Grouper,
	resource::Resource,
	rule::{Rule, Stage},
	sorter::Sorter,
};
use std::{collections::HashMap, sync::Arc};

/// Represents the data flowing through the pipeline.
/// It tracks the current set of file batches and the sequence of
/// groupers that have been applied to create them.
#[derive(Debug)]
pub struct PipelineStream {
	/// The current data, always represented as a list of batches.
	/// An "ungrouped" state is simply a Vec with one Batch.
	pub batches: HashMap<String, Batch>,
	/// The ordered stack of groupers that have been applied.
	pub groupers: Vec<Box<dyn Grouper>>,
	pub sorters: Vec<Box<dyn Sorter>>,
}

impl PipelineStream {
	/// Creates a new stream with a single batch of files and no groupings.
	pub fn new(files: Vec<Arc<Resource>>) -> Self {
		Self {
			batches: HashMap::from([("root".into(), Batch::initial(files))]),
			groupers: Vec::new(),
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

	/// Re-applies the entire stack of stored groupers to a new set of files.
	/// This is the key to maintaining a consistent state.
	pub async fn regroup(&self, files: Vec<Arc<Resource>>) -> Result<HashMap<String, Batch>, anyhow::Error> {
		let mut current_batches = HashMap::from([("root".into(), Batch::initial(files))]);

		for grouper in &self.groupers {
			let mut next_level_batches = HashMap::new();
			for (parent_name, parent_batch) in &current_batches {
				let named_batches_map = grouper.group(parent_batch).await?;
				for (new_key_part, mut sub_batch) in named_batches_map {
					let new_name = if parent_name == "root" {
						new_key_part.clone()
					} else {
						format!("{parent_name}.{new_key_part}")
					};
					sub_batch.context.extend(parent_batch.context.clone());
					sub_batch.context.insert(grouper.name().to_string(), new_key_part.clone());
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

impl Pipeline {
	pub fn new(rule: Rule) -> Self {
		Self {
			stages: rule.pipeline,
			stream: PipelineStream::new(Vec::new()), // Start with no files
		}
	}

	pub async fn run(mut self, ctx: &ExecutionContext<'_>) -> Result<PipelineStream, Error> {
		for stage in self.stages.into_iter() {
			match stage {
				Stage::Search { location, source } => {
					let scope = ExecutionScope::new_location_scope(source.clone(), &location);
					let ctx = ctx.with_scope(scope);
					let new_files = location.backend.discover(&location, &ctx).await?;
					if location.mode.is_append() {
						let mut all_files = self.stream.all_files();
						all_files.extend(new_files);
						self.stream.batches = self.stream.regroup(all_files).await?;
						self.stream.resort().await;
					} else {
						self.stream = PipelineStream::new(new_files);
					}
				}
				Stage::Split { splitter, .. } => {
					let mut next_stream_batches = HashMap::new();
					for (parent_name, parent_batch) in &self.stream.batches {
						let split_batches = splitter.split(parent_batch).await?;

						for (new_key_part, mut sub_batch) in split_batches {
							let new_name = if parent_name == "root" {
								new_key_part.clone()
							} else {
								format!("{parent_name}.{new_key_part}")
							};
							sub_batch.context.extend(parent_batch.context.clone());
							next_stream_batches.insert(new_name, sub_batch);
						}
					}
					self.stream.batches = next_stream_batches;
					self.stream.resort().await;
				}
				Stage::Group { grouper, .. } => {
					let mut next_level_batches = HashMap::new();
					for (parent_name, parent_batch) in &self.stream.batches {
						let named_batches_map = grouper.group(parent_batch).await?;
						for (new_key_part, mut sub_batch) in named_batches_map {
							let new_name = if parent_name == "root" {
								new_key_part.clone()
							} else {
								format!("{parent_name}.{new_key_part}")
							};
							sub_batch.context.extend(parent_batch.context.clone());
							sub_batch.context.insert(grouper.name().to_string(), new_key_part);
							next_level_batches.insert(new_name, sub_batch);
						}
					}
					self.stream.batches = next_level_batches;
					self.stream.groupers.push(grouper);
					self.stream.resort().await;
				}
				Stage::Sort { sorter, .. } => {
					self.stream.sorters.push(sorter);
					self.stream.resort().await;
				}
				Stage::Filter { filter, source } => {
					let mut next_batches = HashMap::new();
					match filter.execution_model() {
						ExecutionModel::Batch => {
							for (name, batch) in &self.stream.batches {
								let scope = ExecutionScope::new_batch_scope(source.clone(), batch);
								let batch_ctx = ctx.with_scope(scope);
								let passed_files = filter.filter(&batch_ctx).await?;
								if !passed_files.is_empty() {
									next_batches.insert(name.clone(), Batch {
										files: passed_files,
										context: batch.context.clone(),
									});
								}
							}
						}
						ExecutionModel::Single => {
							for (name, batch) in &self.stream.batches {
								let mut futs = Vec::new();
								for resource in &batch.files {
									let resource_clone = resource.clone();
									let meta = source.clone();
									let filter = filter.clone();
									let fut = async move {
										let scope = ExecutionScope::new_resource_scope(meta.clone(), resource_clone);
										let ctx = ctx.with_scope(scope);
										filter.filter(&ctx).await
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
				Stage::Action { action, source } => {
					let mut next_stream_batches = HashMap::new(); // This will hold the new batches

					for (name, batch) in &self.stream.batches {
						let mut current_batch_next_files = Vec::new(); // Files for the current batch

						match action.execution_model() {
							ExecutionModel::Batch => {
								let scope = ExecutionScope::new_batch_scope(source.clone(), batch);
								let batch_ctx = ctx.with_scope(scope);
								let receipt = action.commit(&batch_ctx).await?;
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
										let ctx = ctx.with_scope(scope);
										action.commit(&ctx).await
									};
									futs.push(fut);
								}
								let receipts: Vec<Receipt> = future::try_join_all(futs).await?;
								for receipt in receipts {
									current_batch_next_files.extend(receipt.next);
								}
							}
						}

						// Create a new batch with the original name and the collected files
						if !current_batch_next_files.is_empty() {
							next_stream_batches.insert(name.clone(), Batch {
								files: current_batch_next_files,
								context: batch.context.clone(), // Inherit context
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
				Stage::Select { selector, .. } => {
					let mut next_batches = HashMap::new();
					for (name, batch) in &self.stream.batches {
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
