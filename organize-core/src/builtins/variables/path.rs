use crate::{builtins::variables::hash::Hash, context::ExecutionContext, templates::prelude::*};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Copy, Default, PartialEq, Eq)]
pub struct Path;

#[async_trait]
#[typetag::serde(name = "path")]
impl Variable for Path {
	fn name(&self) -> String {
		self.typetag_name().to_string()
	}

	async fn compute(&self, parts: &[String], ctx: &ExecutionContext<'_>) -> Result<VariableOutput, TemplateError> {
		if let Some(next) = parts.iter().next() {
			match next.as_str() {
				"hash" => Ok(VariableOutput::Lazy(Box::new(Hash))),
				_ => Err(TemplateError::UnknownVariable),
			}
		} else {
			Ok(VariableOutput::Value(serde_json::to_value(ctx.scope.resource.as_path())?))
		}
	}
}
