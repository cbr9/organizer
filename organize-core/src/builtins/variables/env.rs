use std::env;

use crate::{context::ExecutionContext, errors::Error, templates::prelude::*};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Eq)]
pub struct Env(String);

#[async_trait]
#[typetag::serde(name = "env")]
impl Variable for Env {
	fn name(&self) -> String {
		self.typetag_name().to_string()
	}

	async fn compute(&self, _ctx: &ExecutionContext<'_>) -> Result<serde_json::Value, Error> {
		let var = env::var(&self.0).map_err(|e| {
			Error::TemplateError(TemplateError::UndefinedVariable {
				variable: self.name(),
				fields: vec![self.0.clone()],
				source: e,
			})
		})?;
		Ok(serde_json::to_value(var)?)
	}
}
