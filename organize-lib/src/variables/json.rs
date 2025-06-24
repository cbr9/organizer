use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
	config::{context::ExecutionContext, variables::Variable},
	templates::template::Template,
};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct TeraVariable {
	pub name: String,
	pub value: serde_json::Value,
}

#[async_trait]
#[typetag::serde(name = "json")]
impl Variable for TeraVariable {
	fn templates(&self) -> Vec<&Template> {
		vec![]
	}

	fn name(&self) -> String {
		self.name.clone()
	}

	async fn compute(&self, _ctx: &ExecutionContext<'_>) -> Result<tera::Value> {
		Ok(self.value.clone())
	}
}
