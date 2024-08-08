use std::sync::Mutex;

use filters::{Extension, Filename, Mime, Parent, Stem};
use lazy_static::lazy_static;
use serde::Deserialize;
use tera::{Context, Tera};

pub mod filters;

lazy_static! {
	pub static ref TERA: Mutex<Tera> = {
		let mut tera = Tera::default();
		tera.register_filter("parent", Parent);
		tera.register_filter("stem", Stem);
		tera.register_filter("filename", Filename);
		tera.register_filter("extension", Extension);
		tera.register_filter("mime", Mime);
		Mutex::new(tera)
	};
}

#[derive(Deserialize, Default, Debug, PartialEq, Clone)]
pub struct Template(pub String);

impl Template {
	pub fn expand(&self, context: &Context) -> tera::Result<String> {
		TERA.lock().unwrap().render_str(&self.0, context)
	}
}

impl From<Template> for String {
	fn from(val: Template) -> Self {
		val.0
	}
}
impl From<String> for Template {
	fn from(val: String) -> Self {
		Template(val)
	}
}
