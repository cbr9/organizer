use std::sync::Mutex;

use filters::{Extension, Filename, Mime, Parent, Stem};
use lazy_static::lazy_static;
use tera::{Context, Tera};

use crate::path::get_env_context;
pub mod filters;

lazy_static! {
	pub static ref CONTEXT: Mutex<Context> = Mutex::new(get_env_context());
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
