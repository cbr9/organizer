use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
	context::ExecutionContext,
	templates::{
		engine::TemplateError,
		prelude::{Template, Variable, VariableOutput},
	},
};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct TeraVariable {
	pub name: String,
	pub value: Template,
}

#[async_trait]
#[typetag::serde(name = "template")]
impl Variable for TeraVariable {
	fn name(&self) -> String {
		self.name.clone()
	}

	async fn compute(&self, _parts: &[String], ctx: &ExecutionContext<'_>) -> Result<VariableOutput, TemplateError> {
		let value = ctx.services.templater.render(&self.value, ctx).await?;
		Ok(VariableOutput::Value(serde_json::to_value(value)?))
	}
}
