use serde::Deserialize;
use tera::Context;

use crate::{
	config::filters::regex::{deserialize_regex, RegularExpression},
	templates::TERA,
};

use super::AsVariable;

#[derive(Debug, Clone, Deserialize)]
pub struct RegexVariable {
	#[serde(deserialize_with = "deserialize_regex")]
	pub pattern: regex::Regex,
	pub input: String,
}

impl AsVariable for RegularExpression {
	fn register(&self, context: &mut Context) {
		let input = TERA.lock().unwrap().render_str(&self.input, context).unwrap();
		if let Some(captures) = self.pattern.captures(&input) {
			for name in self.pattern.capture_names().flatten() {
				if let Some(r#match) = captures.name(name) {
					context.insert(name, r#match.as_str());
				}
			}
		}
	}
}
