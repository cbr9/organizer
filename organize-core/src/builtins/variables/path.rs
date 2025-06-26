use crate::{builtins::variables::hash::Hash, context::ExecutionContext, errors::Error, templates::prelude::*};
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

	async fn compute(&self, parts: &[String], ctx: &ExecutionContext<'_>) -> Result<VariableOutput, Error> {
		let resource = ctx.scope.resource()?;
		if let Some(next) = parts.iter().next() {
			match next.as_str() {
				"hash" => Ok(VariableOutput::Lazy(Box::new(Hash))),
				"stem" => Ok(VariableOutput::Value(serde_json::to_value(
					resource.as_path().file_stem().unwrap().to_string_lossy(),
				)?)),
				"extension" => Ok(VariableOutput::Value(serde_json::to_value(
					resource.as_path().extension().unwrap().to_string_lossy(),
				)?)),
				other => Err(TemplateError::UnknownVariable(other.into()))?,
			}
		} else {
			Ok(VariableOutput::Value(serde_json::to_value(resource.as_path())?))
		}
	}
}
