
use serde::Deserialize;
use tera::Context;

use crate::templates::TERA;

use super::AsVariable;

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct SimpleVariable {
	name: String,
	value: String,
}
impl AsVariable for SimpleVariable {
	fn register(&self, context: &mut Context) {
		let value = TERA.lock().unwrap().render_str(&self.value, context).unwrap();
		context.insert(&self.name, &value);
	}
}
