use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::config::{context::ExecutionContext, variables::Variable};

#[derive(Debug, Clone, Deserialize, Serialize, Copy, Default, PartialEq, Eq)]
pub struct Path;

#[async_trait]
#[typetag::serde(name = "path")]
impl Variable for Path {
	async fn compute(&self, ctx: &ExecutionContext<'_>) -> anyhow::Result<tera::Value> {
		Ok(serde_json::to_value(ctx.scope.resource.as_path())?)
	}
}
