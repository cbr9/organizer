use serde::{Deserialize, Serialize};
use tera::Context;

use crate::{
	config::filters::regex::Regex,
	templates::{template::Template, TemplateEngine},
};

use super::Variable;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegexVariable {
	pub pattern: Regex,
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
	fn templates(&self) -> Vec<&Template> {
		vec![&self.input]
	}

	fn register(&self, template_engine: &TemplateEngine, context: &mut Context) {
		if let Some(input) = template_engine.render(&self.input, context).unwrap_or_default() {
			if let Some(captures) = self.pattern.captures(&input) {
				for name in self.pattern.capture_names().flatten() {
					if let Some(r#match) = captures.name(name) {
						context.insert(name, r#match.as_str());
					}
				}
			}
		}
	}
}
