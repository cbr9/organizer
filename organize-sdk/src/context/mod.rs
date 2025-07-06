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
pub struct ExecutionContext {
	pub services: Arc<RunServices>,
	pub scope: ExecutionScope,
	pub settings: Arc<RunSettings>,
}

impl ExecutionContext {
	pub fn with_scope(&self, scope: ExecutionScope) -> ExecutionContext {
		Self {
			services: self.services.clone(),
			scope,
			settings: self.settings.clone(),
		}
	}

	pub fn with_resource(&self, resource: &Arc<Resource>) -> Result<ExecutionContext, Error> {
		let scope = ExecutionScope::new_resource_scope(self.scope.rule()?, resource.clone());
		Ok(self.with_scope(scope))
	}
}
