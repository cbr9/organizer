use crate::{context::ExecutionContext, errors::Error, templates::prelude::*};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Eq)]
pub struct Rule(Vec<String>);

#[async_trait]
#[typetag::serde(name = "rule")]
impl Variable for Rule {
	fn name(&self) -> String {
		self.typetag_name().to_string()
	}

	async fn compute(&self, ctx: &ExecutionContext<'_>) -> Result<serde_json::Value, Error> {
		let rule = ctx.scope.rule()?;
		let mut parts = self.0.iter().cloned();
		if let Some(next) = parts.next() {
			match next.as_str() {
				"id" => Ok(serde_json::to_value(rule.id.clone().unwrap_or("<undefined>".to_string()))?),
				"tags" => {
					let next = parts.next().unwrap();
					let int = next.parse::<usize>().unwrap();
					Ok(serde_json::to_value(&rule.tags.get(int).unwrap_or(&"<undefined>".to_string()))?)
				}
				other => Err(TemplateError::InvalidField {
					variable: self.name(),
					field: other.to_string(),
				})?,
			}
		} else {
			Err(Error::TemplateError(TemplateError::RequiredField {
				variable: self.name(),
				fields: vec!["id".into()],
			}))
		}
	}
}
