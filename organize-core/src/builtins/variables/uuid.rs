use crate::{builtins::variables::hash::Hash, context::ExecutionContext, errors::Error, templates::prelude::*};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Copy, Default, PartialEq, Eq)]
pub struct Uuid;

#[async_trait]
#[typetag::serde(name = "uuid")]
impl Variable for Uuid {
	fn name(&self) -> String {
		self.typetag_name().to_string()
	}

	async fn compute(&self, _parts: &[String], _ctx: &ExecutionContext<'_>) -> Result<VariableOutput, Error> {
		let id = uuid::Uuid::new_v4().to_string();
		Ok(VariableOutput::Value(serde_json::to_value(id)?))
	}
}
