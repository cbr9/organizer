pub(crate) mod path {
	mod expand;
	mod is_hidden;
	mod update;
	pub(crate) use expand::*;
	pub(crate) use is_hidden::*;
	pub(crate) use update::*;
}
pub(crate) mod string {
	mod capitalize;
	mod placeholder;
	pub(crate) use capitalize::*;
	pub(crate) use placeholder::*;
}
pub mod data;
pub mod file;
pub mod register;
pub mod utils;

#[macro_use]
extern crate strum_macros;

pub const PROJECT_NAME: &str = "organize";
