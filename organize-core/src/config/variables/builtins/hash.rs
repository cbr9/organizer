use std::{path::PathBuf, sync::Arc};

use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::config::{context::ExecutionContext, variables::Variable};

// -- Hash Variable (Stateful with Cache) --------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hash {
	cache: Arc<DashMap<PathBuf, String>>,
}

impl Eq for Hash {}
impl PartialEq for Hash {
	fn eq(&self, other: &Self) -> bool {
		self.cache
			.iter()
			.zip(other.cache.iter())
			.all(|(a, b)| a.key() == b.key() && a.value() == b.value())
	}
}

impl Default for Hash {
    fn default() -> Self {
        Self::new()
    }
}

impl Hash {
	pub fn new() -> Self {
		Self {
			cache: Arc::new(DashMap::new()),
		}
	}
}

#[async_trait]
#[typetag::serde(name = "hash")]
impl Variable for Hash {
	async fn compute(&self, ctx: &ExecutionContext<'_>) -> anyhow::Result<tera::Value> {
		let path = ctx.scope.resource.as_path();
		if let Some(entry) = self.cache.get(path) {
			return Ok(serde_json::to_value(entry.value())?);
		}

		let content = tokio::fs::read(path).await?;
		let hash = sha256::digest(&content);

		self.cache.insert(path.to_path_buf(), hash.clone());
		Ok(serde_json::to_value(hash)?)
	}
}
