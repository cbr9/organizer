use crate::config::{context::ExecutionContext, variables::Variable};
use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{path::PathBuf, sync::Arc, time::UNIX_EPOCH};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Metadata {
	cache: Arc<DashMap<PathBuf, Value>>,
}

impl Eq for Metadata {}
impl PartialEq for Metadata {
	fn eq(&self, other: &Self) -> bool {
		self.cache
			.iter()
			.zip(other.cache.iter())
			.all(|(a, b)| a.key() == b.key() && a.value() == b.value())
	}
}

impl Default for Metadata {
    fn default() -> Self {
        Self::new()
    }
}

impl Metadata {
	pub fn new() -> Self {
		Self {
			cache: Arc::new(DashMap::new()),
		}
	}
}

// A helper struct to create a nice, serializable representation of file metadata.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
struct FileMetadata {
	is_file: bool,
	is_dir: bool,
	len: u64,
	modified: u64,
	created: u64,
	accessed: u64,
}

#[async_trait]
#[typetag::serde(name = "metadata")]
impl Variable for Metadata {
	async fn compute(&self, ctx: &ExecutionContext<'_>) -> Result<Value> {
		let path = ctx.scope.resource.as_path();
		if let Some(entry) = self.cache.get(path) {
			return Ok(entry.value().clone());
		}

		let metadata = tokio::fs::metadata(path).await?;
		let serializable = FileMetadata {
			is_file: metadata.is_file(),
			is_dir: metadata.is_dir(),
			len: metadata.len(),
			modified: metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs(),
			created: metadata.created()?.duration_since(UNIX_EPOCH)?.as_secs(),
			accessed: metadata.accessed()?.duration_since(UNIX_EPOCH)?.as_secs(),
		};
		let value = serde_json::to_value(serializable)?;

		self.cache.insert(path.to_path_buf(), value.clone());
		Ok(value)
	}
}
