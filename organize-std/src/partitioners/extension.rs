use async_trait::async_trait;
use organize_sdk::{engine::batch::Batch, error::Error, plugins::partitioner::Partitioner};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionPartitioner;

#[async_trait]
#[typetag::serde(name = "extension")]
impl Partitioner for ExtensionPartitioner {
	fn name(&self) -> &str {
		self.typetag_name()
	}

	async fn partition(&self, batch: &Batch) -> Result<HashMap<String, Batch>, Error> {
		let mut groups: HashMap<String, Batch> = HashMap::new();
		for resource in &batch.files {
			let extension = resource
				.path
				.extension()
				.and_then(|s| s.to_str())
				.unwrap_or("no_extension")
				.to_string();
			groups
				.entry(extension.clone())
				.or_insert_with(Batch::new)
				.files
				.push(resource.clone());
		}
		Ok(groups)
	}
}
