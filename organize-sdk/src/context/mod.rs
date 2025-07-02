pub mod scope;
pub mod services;
pub mod settings;

use crate::{
    context::{scope::ExecutionScope, services::RunServices, settings::RunSettings},
};

/// The top-level context object, composed of the three distinct categories of information.
#[derive(Clone, Debug)]
pub struct ExecutionContext<'a> {
	pub services: &'a RunServices,
	pub scope: ExecutionScope<'a>,
	pub settings: &'a RunSettings,
}

impl<'a> ExecutionContext<'a> {
	pub fn with_scope(&'a self, scope: ExecutionScope<'a>) -> ExecutionContext<'a> {
		Self {
			services: self.services,
			scope,
			settings: self.settings,
		}
	}
}