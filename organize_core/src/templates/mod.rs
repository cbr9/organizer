use std::sync::Mutex;

use filters::{
	misc::{hash, mime},
	path::{extension, filename, parent, stem},
	size::size,
};
use lazy_static::lazy_static;
use serde::Deserialize;
use tera::{Context, Tera};

pub mod filters;

lazy_static! {
	static ref TERA: Mutex<Tera> = {
		let mut tera = Tera::default();
		tera.register_filter("parent", parent);
		tera.register_filter("stem", stem);
		tera.register_filter("filename", filename);
		tera.register_filter("extension", extension);
		tera.register_filter("mime", mime);
		tera.register_filter("filesize", size);
		tera.register_filter("hash", hash);
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
