use std::sync::{LazyLock, Mutex};

use filters::{
	content::file_content,
	misc::{hash, mime},
	path::{extension, filename, parent, stem},
	size::size,
};
use serde::{Deserialize, Serialize};
use tera::{Context, Tera};

pub mod filters;

static TERA: LazyLock<Mutex<Tera>> = LazyLock::new(|| {
	let mut tera = Tera::default();
	tera.register_filter("parent", parent);
	tera.register_filter("stem", stem);
	tera.register_filter("filename", filename);
	tera.register_filter("extension", extension);
	tera.register_filter("mime", mime);
	tera.register_filter("filesize", size);
	tera.register_filter("hash", hash);
	tera.register_filter("filecontent", file_content);
	Mutex::new(tera)
});

#[derive(Deserialize, Serialize, Default, Debug, Eq, PartialEq, Clone)]
pub struct Template(pub String);

impl Template {
	#[tracing::instrument(err(Debug))]
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
