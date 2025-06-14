use serde::{Deserialize, Serialize};
use tera::Context;

use crate::templates::Template;

use super::Variable;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct SimpleVariable {
	name: String,
	value: Template,
}

#[typetag::serde(name = "simple")]
impl Variable for SimpleVariable {
	fn register(&self, context: &mut Context) {
		let value = &self.value.render(context).unwrap();
		context.insert(&self.name, &value);
	}
}
