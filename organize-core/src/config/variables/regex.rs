use tera::Context;

use crate::{
	config::filters::regex::RegularExpression,
	templates::{template::Template, Templater},
};

use super::Variable;

#[typetag::serde(name = "regex")]
impl Variable for RegularExpression {
	fn templates(&self) -> Vec<&Template> {
		vec![&self.input]
	}

	fn register(&self, template_engine: &Templater, context: &mut Context) {
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
