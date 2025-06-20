
use serde::{Deserialize, Serialize};
use tera::{Map, Value};

use crate::{
	config::{context::ExecutionContext, filters::regex::Regex},
	templates::{template::Template, Context},
};

use super::Variable;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct RegularExpression {
	pub pattern: Regex,
	#[serde(default)]
	pub input: Template,
	pub name: String,
}

#[typetag::serde(name = "regex")]
impl Variable for RegularExpression {
	fn name(&self) -> &str {
		&self.name
	}

	fn templates(&self) -> Vec<&Template> {
		vec![&self.input]
	}

	/// Computes the value of the variable. For Regex, this means running the
	/// match and returning an object/map of the named captures.
	fn compute(&self, ctx: &ExecutionContext) -> anyhow::Result<tera::Value> {
		let context = Context::new(ctx);
		let input = match ctx.services.templater.render(&self.input, &context)? {
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

		Ok(Value::Object(capture_map))
	}
}
