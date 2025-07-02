use async_trait::async_trait;
use organize_sdk::{engine::batch::Batch, error::Error, plugins::partitioner::Partitioner};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, iter::FromIterator};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatioPartitioner {
	/// Captures arbitrary split names and their corresponding ratios,
	/// e.g., `test = 0.2`, `train = 0.8`.
	#[serde(flatten)]
	pub ratios: HashMap<String, f64>,
}

impl Eq for RatioPartitioner {}

impl PartialEq for RatioPartitioner {
	fn eq(&self, other: &Self) -> bool {
		// First, check if the number of entries is the same
		if self.ratios.len() != other.ratios.len() {
			return false;
		}

		let epsilon = 1e-9; // Define your tolerance for f64 comparison

		// Iterate over the keys and compare values
		for (key, self_value) in &self.ratios {
			if let Some(other_value) = other.ratios.get(key) {
				// Check if the absolute difference is within epsilon
				if (self_value - other_value).abs() >= epsilon {
					return false; // Values are not "nearly equal"
				}
			} else {
				return false; // Key exists in self but not in other
			}
		}
		true // All keys and values are "nearly equal"
	}
}

#[async_trait]
#[typetag::serde(name = "ratio")]
impl Partitioner for RatioPartitioner {
	fn name(&self) -> &str {
		self.typetag_name()
	}

	async fn partition(&self, batch: &Batch) -> Result<HashMap<String, Batch>, Error> {
		// 1. Validate that the ratios sum to approximately 1.0
		let total_ratio: f64 = self.ratios.values().sum();
		if (total_ratio - 1.0).abs() > 1e-9 {
			return Err(Error::Other(anyhow::anyhow!(
				"Ratios for splitter must sum to 1.0, but they sum to {}",
				total_ratio
			)));
		}

		// 2. Calculate and distribute files
		let mut result_batches = Vec::new();
		let mut remaining_files = batch.files.as_slice();
		let total_files = batch.files.len();

		// The user is responsible for shuffling, so we process files in the given order.
		for (name, &ratio) in &self.ratios {
			let num_to_take = (total_files as f64 * ratio).round() as usize;
			let num_to_take = num_to_take.min(remaining_files.len());
			let (split_files_slice, rest) = remaining_files.split_at(num_to_take);

			let mut new_batch = Batch::new();
			new_batch.files = split_files_slice.to_vec();

			result_batches.push((name.clone(), new_batch));
			remaining_files = rest;
		}

		// 3. To ensure deterministic behavior, distribute any leftover files (due to rounding)
		//    to the last batch.
		if !remaining_files.is_empty() {
			if let Some(last_batch) = result_batches.last_mut() {
				last_batch.1.files.extend_from_slice(remaining_files);
			}
		}

		let batches = HashMap::from_iter(result_batches);

		Ok(batches)
	}
}
