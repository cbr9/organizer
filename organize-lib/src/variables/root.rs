use crate::config::{context::ExecutionContext, variables::Variable};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Serialize, Copy, Default, PartialEq, Eq)]
pub struct Root;

#[async_trait]
#[typetag::serde(name = "root")]
impl Variable for Root {
	async fn compute(&self, ctx: &ExecutionContext<'_>) -> Result<Value> {
		Ok(serde_json::to_value(&ctx.scope.folder.path)?)
	}
}
