use crate::{
	context::ExecutionContext,
	errors::Error,
	templates::variable::{Variable, VariableOutput},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// -- Hash Variable (Stateful with Cache) --------------------------------------
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Hash;

#[async_trait]
#[typetag::serde(name = "hash")]
impl Variable for Hash {
	fn name(&self) -> String {
		self.typetag_name().to_string()
	}

	async fn compute(&self, _parts: &[String], ctx: &ExecutionContext<'_>) -> Result<VariableOutput, Error> {
		let resource = ctx.scope.resource()?;
		let hash = resource.get_hash().await;
		Ok(VariableOutput::Value(serde_json::to_value(hash)?))
	}
}
