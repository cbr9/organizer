use serde::{Deserialize, Serialize};
use tera::Context;

use crate::templates::Template;

use super::AsVariable;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct SimpleVariable {
	name: String,
	value: Template,
}
impl AsVariable for SimpleVariable {
	fn register(&self, context: &mut Context) {
		let value = &self.value.render(context).unwrap();
		context.insert(&self.name, &value);
	}
}
