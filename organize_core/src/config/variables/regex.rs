use serde::{Deserialize, Serialize};
use tera::Context;

use crate::templates::Template;

use super::Variable;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegexVariable {
	#[serde(deserialize_with = "serde_regex::deserialize", serialize_with = "serde_regex::serialize")]
	pub pattern: regex::Regex,
	pub input: Template,
}

impl PartialEq for RegexVariable {
	fn eq(&self, other: &Self) -> bool {
		self.pattern.as_str() == other.pattern.as_str() && self.input == other.input
	}
}

impl Eq for RegexVariable {}

#[typetag::serde(name = "regex")]
impl Variable for RegexVariable {
	fn register(&self, context: &mut Context) {
		let input = self.input.render(context).unwrap();
		if let Some(captures) = self.pattern.captures(&input) {
			for name in self.pattern.capture_names().flatten() {
				if let Some(r#match) = captures.name(name) {
					context.insert(name, r#match.as_str());
				}
			}
		}
	}
}
