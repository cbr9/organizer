extern crate strum_macros;

pub mod path {
	pub use expand::*;
	pub use is_hidden::*;
	pub use prepare::*;

	mod expand;
	mod is_hidden;
	mod prepare;
}

pub mod config;
pub mod logger;
pub mod templates;
pub mod utils;

pub const PROJECT_NAME: &str = "organize";
