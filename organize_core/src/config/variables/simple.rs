use std::borrow::BorrowMut;

use serde::Deserialize;

use crate::templates::{CONTEXT, TERA};

use super::AsVariable;

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct SimpleVariable {
	name: String,
	value: String,
}
impl AsVariable for SimpleVariable {
	fn register(&self) {
		let mut ctx = CONTEXT.lock().unwrap();
		let value = TERA.lock().unwrap().render_str(&self.value, ctx.borrow_mut()).unwrap();
		ctx.insert(&self.name, &value);
	}
}
