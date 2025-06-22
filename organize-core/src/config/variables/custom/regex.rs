use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tera::{Map, Value};

use crate::{
	config::{context::ExecutionContext, filters::regex::Regex, variables::Variable},
	templates::template::Template,
};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct RegularExpression {
	pub pattern: Regex,
	#[serde(default)]
	pub input: Template,
	pub name: String,
}

#[async_trait]
#[typetag::serde(name = "regex")]
impl Variable for RegularExpression {
	fn name(&self) -> Option<&str> {
		Some(&self.name)
	}

	fn templates(&self) -> Vec<&Template> {
		vec![&self.input]
	}

	async fn compute(&self, ctx: &ExecutionContext<'_>) -> anyhow::Result<tera::Value> {
		let input = match ctx.services.templater.render(&self.input, ctx).await? {
			Some(value) => value,
			None => return Ok(Value::Object(Map::new())),
		};

		let Some(captures) = self.pattern.captures(&input) else {
			return Ok(Value::Object(Map::new()));
		};

		let mut capture_map = Map::new();

		for name in self.pattern.capture_names().flatten() {
			if let Some(match_value) = captures.name(name) {
				capture_map.insert(name.to_string(), Value::String(match_value.as_str().to_string()));
			}
		}

		for (i, match_value_opt) in captures.iter().enumerate() {
			if let Some(match_value) = match_value_opt {
				capture_map.insert(format!("${i}"), Value::String(match_value.as_str().to_string()));
			}
		}
		Ok(Value::Object(capture_map))
	}
}
