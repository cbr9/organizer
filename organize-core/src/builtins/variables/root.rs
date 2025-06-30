use anyhow::Result;
use async_trait::async_trait;
use path_clean::PathClean;
use serde::{Deserialize, Serialize};

use crate::{
	context::ExecutionContext,
	errors::Error,
	templates::prelude::Variable,
};

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Eq)]
pub struct Root;

#[async_trait]
#[typetag::serde(name = "root")]
impl Variable for Root {
	fn name(&self) -> String {
		self.typetag_name().to_string()
	}

	async fn compute(&self, ctx: &ExecutionContext<'_>) -> Result<serde_json::Value, Error> {
		let root = ctx.scope.root()?;
		Ok(serde_json::to_value(root.clean())?)
	}
}
