use anyhow::Result;
use async_trait::async_trait;
use organize_sdk::{engine::batch::Batch, error::Error, plugins::selector::Selector};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstSelector {
	pub n: usize,
}

#[async_trait]
#[typetag::serde(name = "first")]
impl Selector for FirstSelector {
	async fn select(&self, batch: &Batch) -> Result<Batch, Error> {
		let mut selected_batch = Batch::new();
		selected_batch.files = batch.files.iter().take(self.n).cloned().collect();
		// The context from the original batch is not preserved by default,
		// as it might not be relevant to the new, smaller batch.
		Ok(selected_batch)
	}
}
