use crate::{
	context::ExecutionContext,
	errors::Error,
	templates::{engine::TemplateError, variable::Variable},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Hash;

#[async_trait]
#[typetag::serde(name = "hash")]
impl Variable for Hash {
	fn name(&self) -> String {
		self.typetag_name().to_string()
	}

	async fn compute(&self, ctx: &ExecutionContext<'_>) -> Result<serde_json::Value, Error> {
		let resource = ctx.scope.resource()?;
		let hash = resource.get_hash().await;
		Ok(serde_json::to_value(hash)?)
	}
}
