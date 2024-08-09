use std::sync::Mutex;

use filters::{Extension, Filename, Mime, Parent, Stem};
use lazy_static::lazy_static;
use serde::Deserialize;
use tera::{Context, Tera};

pub mod filters;

lazy_static! {
	static ref TERA: Mutex<Tera> = {
		let mut tera = Tera::default();
		tera.register_filter("parent", Parent);
		tera.register_filter("stem", Stem);
		tera.register_filter("filename", Filename);
		tera.register_filter("extension", Extension);
		tera.register_filter("mime", Mime);
		Mutex::new(tera)
	};
}

#[derive(Deserialize, Default, Debug, Eq, PartialEq, Clone)]
pub struct Template(pub String);

impl Template {
	#[tracing::instrument(ret(level = "debug"), err(Debug))]
	pub fn render(&self, context: &Context) -> tera::Result<String> {
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

impl From<&str> for Template {
	fn from(val: &str) -> Self {
		Template(val.to_string())
	}
}
