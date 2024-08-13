use std::str::FromStr;

use serde::Deserialize;
use tera::Context;

use crate::templates::Template;

use super::AsVariable;

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct SimpleVariable {
	name: String,
	value: Template,
}
impl AsVariable for SimpleVariable {
	fn register(&self, context: &mut Context) {
		let value = tera::Value::from_str(&self.value.render(context).unwrap()).unwrap();
		context.insert(&self.name, &value);
	}
}
