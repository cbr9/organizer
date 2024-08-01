use std::sync::{Arc, Mutex};

use lazy_static::lazy_static;
use rusqlite::Connection;

extern crate strum_macros;

pub(crate) mod path {
	pub(crate) use expand::*;
	pub(crate) use is_hidden::*;
	pub(crate) use prepare::*;

	mod expand;
	mod is_hidden;
	mod prepare;
}

pub mod config;
pub mod file;
pub mod logger;
pub mod utils;

pub const PROJECT_NAME: &str = "organize";

lazy_static! {
	pub static ref DB: Arc<Mutex<Connection>> = Arc::new(Mutex::new(
		Connection::open(dirs_next::data_local_dir().unwrap().join("organize").join("organize.db")).unwrap()
	));
}
