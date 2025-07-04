pub mod scope;
pub mod services;
pub mod settings;

use std::sync::Arc;

use crate::{
	context::{scope::ExecutionScope, services::RunServices, settings::RunSettings},
	error::Error,
	resource::Resource,
};

/// The top-level context object, composed of the three distinct categories of information.
#[derive(Clone)]
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

	pub fn with_resource(&'a self, resource: &Arc<Resource>) -> Result<ExecutionContext<'a>, Error> {
		let scope = ExecutionScope::new_resource_scope(self.scope.rule()?, resource.clone());
		Ok(self.with_scope(scope))
	}
}
