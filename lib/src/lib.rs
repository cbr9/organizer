pub mod config;
pub mod path {
	mod expand;
	mod get_rules;
	mod is_hidden;
	mod update;
	pub use expand::*;
	pub use get_rules::*;
	pub use is_hidden::*;
	pub use update::*;
}
pub mod string {
	mod capitalize;
	mod placeholder;
	pub use capitalize::*;
	pub use placeholder::*;
}
pub mod register;
pub mod settings;
pub mod utils;

pub const PROJECT_NAME: &str = "organize";
