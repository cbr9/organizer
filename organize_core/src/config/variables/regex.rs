use std::str::FromStr;

use serde::Deserialize;
use tera::{Context, Value};

use crate::templates::Template;

use super::AsVariable;

#[derive(Debug, Clone, Deserialize)]
pub struct RegexVariable {
	#[serde(deserialize_with = "serde_regex::deserialize")]
	pub pattern: regex::Regex,
	pub input: Template,
}

impl PartialEq for RegexVariable {
	fn eq(&self, other: &Self) -> bool {
		self.pattern.as_str() == other.pattern.as_str() && self.input == other.input
	}
}

impl AsVariable for RegexVariable {
	fn register(&self, context: &mut Context) {
		let input = self.input.render(context).unwrap();
		if let Some(captures) = self.pattern.captures(&input) {
			for name in self.pattern.capture_names().flatten() {
				if let Some(r#match) = captures.name(name) {
					let value = Value::from_str(r#match.as_str()).unwrap();
					context.insert(name, &value);
				}
			}
		}
	}
}
