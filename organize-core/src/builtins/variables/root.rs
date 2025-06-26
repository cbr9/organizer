use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
	context::ExecutionContext,
	errors::Error,
	templates::prelude::{Variable, VariableOutput},
};

#[derive(Debug, Clone, Deserialize, Serialize, Copy, Default, PartialEq, Eq)]
pub struct Root;

#[async_trait]
#[typetag::serde(name = "root")]
impl Variable for Root {
	fn name(&self) -> String {
		self.typetag_name().to_string()
	}

	async fn compute(&self, _parts: &[String], ctx: &ExecutionContext<'_>) -> Result<VariableOutput, Error> {
		let folder = ctx.scope.folder()?;
		Ok(VariableOutput::Value(serde_json::to_value(&folder.path)?))
	}
}
