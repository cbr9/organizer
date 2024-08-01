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
pub mod logger;
pub mod utils;

pub const PROJECT_NAME: &str = "organize";
