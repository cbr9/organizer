use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
	config::{context::ExecutionContext, variables::Variable},
	templates::template::Template,
};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct TemplateVariable {
	pub name: String,
	pub value: Template,
}

#[async_trait]
#[typetag::serde(name = "template")]
impl Variable for TemplateVariable {
	fn templates(&self) -> Vec<&Template> {
		vec![&self.value]
	}

	fn name(&self) -> &str {
		&self.name
	}

	async fn compute(&self, ctx: &ExecutionContext<'_>) -> Result<tera::Value> {
		let rendered = ctx.services.templater.render(&self.value, ctx).await?;

		// The rendered string is the final value of this variable.
		Ok(serde_json::to_value(rendered)?)
	}
}
