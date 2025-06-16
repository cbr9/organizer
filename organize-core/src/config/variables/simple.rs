use serde::{Deserialize, Serialize};
use tera::Context;

use super::Variable;
use crate::templates::{template::Template, TemplateEngine};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct SimpleVariable {
	pub name: String,
	pub value: Template,
}

#[typetag::serde(name = "simple")]
impl Variable for SimpleVariable {
	fn templates(&self) -> Vec<&Template> {
		vec![&self.value]
	}

	fn register(&self, template_engine: &TemplateEngine, context: &mut Context) {
		let value = template_engine.render(&self.value, context).unwrap();
		context.insert(&self.name, &value);
	}
}
