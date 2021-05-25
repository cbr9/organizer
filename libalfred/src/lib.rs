#[macro_use]
extern crate strum_macros;

pub(crate) mod path {
	pub(crate) use expand::*;
	pub(crate) use is_hidden::*;
	pub(crate) use update::*;

	mod expand;
	mod is_hidden;
	mod update;
}

pub(crate) mod string {
	pub(crate) use capitalize::*;
	pub(crate) use placeholder::*;

	mod capitalize;
	mod placeholder;
}
pub mod data;
pub mod file;
pub mod logger;
pub mod register;
pub mod simulation;
pub mod utils;

pub const PROJECT_NAME: &str = "alfred";
