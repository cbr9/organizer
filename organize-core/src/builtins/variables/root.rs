use anyhow::Result;
use async_trait::async_trait;
use path_clean::PathClean;
use serde::{Deserialize, Serialize};

use crate::{
	context::ExecutionContext,
	errors::Error,
	templates::{engine::TemplateError, prelude::Variable},
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
		let folder = ctx.scope.folder()?;
		Ok(serde_json::to_value(&folder.path.clean())?)
	}
}
