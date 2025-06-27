pub mod engine;
pub mod filter;
pub mod template;
pub mod variable;

pub mod prelude {
	pub use super::{
		engine::{TemplateError, Templater},
		template::Template,
		variable::Variable,
	};
}
