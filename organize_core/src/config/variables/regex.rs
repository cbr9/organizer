use serde::Deserialize;
use tera::Context;

use crate::{
	config::filters::regex::{deserialize_regex, RegularExpression},
	templates::Template,
};

use super::AsVariable;

#[derive(Debug, Clone, Deserialize)]
pub struct RegexVariable {
	#[serde(deserialize_with = "deserialize_regex")]
	pub pattern: regex::Regex,
	pub input: Template,
}

impl AsVariable for RegexVariable {
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
