use anyhow::Result;
use dashmap::DashMap;
use std::{
	any::Any,
	path::{Path, PathBuf},
	sync::Arc,
};

pub mod services;

use crate::{
	batch::Batch,
	context::services::{fs::manager::FileSystemManager, history::Journal},
	errors::Error,
	folder::Location,
	resource::Resource,
	rule::RuleMetadata,
};

#[derive(Debug, Clone)]
pub struct RunServices {
	pub blackboard: Blackboard,
	pub fs: FileSystemManager,
	pub journal: Arc<Journal>,
}

#[derive(Debug, Clone)]
pub struct Blackboard {
	pub scratchpad: Arc<DashMap<String, Box<dyn Any + Send + Sync>>>,
	pub shared_context: Arc<DashMap<String, String>>,
}

impl Default for Blackboard {
	fn default() -> Self {
		Self {
			scratchpad: Arc::new(DashMap::new()),
			shared_context: Arc::new(DashMap::new()),
		}
	}
}

/// A container for run-wide operational settings.
#[derive(Debug, Clone, Copy)]
pub struct RunSettings {
	pub dry_run: bool,
}

/// A read-only "view" into the current position in the configuration tree.
// #[derive(Debug, Clone)]
// pub struct ExecutionScope<'a> {
// 	pub config: &'a Config,
// 	pub rule: Option<&'a Rule>,
// 	pub folder: Option<&'a Folder>,
// 	pub resource: Option<Arc<Resource>>,
// 	pub resources: Option<Vec<Arc<Resource>>>,
// }

#[derive(Debug, Clone)]
pub enum ExecutionScope<'a> {
	Rule(RuleScope),
	Search(SearchScope<'a>),
	Resource(ResourceScope),
	Batch(BatchScope<'a>),
	Build(BuildScope),
	Blank,
}

impl<'a> ExecutionScope<'a> {
	pub fn new_rule_scope(rule: Arc<RuleMetadata>) -> ExecutionScope<'a> {
		ExecutionScope::Rule(RuleScope { rule })
	}

	pub fn new_location_scope(rule: Arc<RuleMetadata>, location: &'a Location) -> ExecutionScope<'a> {
		ExecutionScope::Search(SearchScope { rule, location })
	}

	pub fn new_resource_scope(rule: Arc<RuleMetadata>, resource: Arc<Resource>) -> ExecutionScope<'a> {
		ExecutionScope::Resource(ResourceScope { rule, resource })
	}

	pub fn new_batch_scope(rule: Arc<RuleMetadata>, batch: &'a Batch) -> ExecutionScope<'a> {
		ExecutionScope::Batch(BatchScope { rule, batch })
	}

	pub fn new_build_scope(root: &Path) -> ExecutionScope<'a> {
		ExecutionScope::Build(BuildScope { root: root.to_path_buf() })
	}

	pub fn rule(&self) -> Result<Arc<RuleMetadata>, Error> {
		match self {
			ExecutionScope::Rule(scope) => Ok(scope.rule.clone()),
			ExecutionScope::Resource(scope) => Ok(scope.rule.clone()),
			ExecutionScope::Batch(scope) => Ok(scope.rule.clone()),
			ExecutionScope::Search(scope) => Ok(scope.rule.clone()),
			_ => Err(Error::OutOfScope("rule".into())),
		}
	}

	pub fn resource(&self) -> Result<Arc<Resource>, Error> {
		match self {
			ExecutionScope::Resource(scope) => Ok(scope.resource.clone()),
			_ => Err(Error::OutOfScope("resource".into())),
		}
	}

	pub fn batch(&self) -> Result<&'a Batch, Error> {
		match self {
			ExecutionScope::Batch(scope) => Ok(scope.batch),
			_ => Err(Error::OutOfScope("batch".into())),
		}
	}

	pub fn root(&self) -> Result<PathBuf, Error> {
		match self {
			ExecutionScope::Search(scope) => Ok(scope.location.path.clone()),
			ExecutionScope::Resource(scope) => Ok(scope.resource.location.path.clone()),
			ExecutionScope::Build(path) => Ok(path.root.clone()), // <-- ADD THIS CASE
			_ => Err(Error::OutOfScope("root".into())),
		}
	}
}

#[derive(Debug, Clone)]
pub struct RuleScope {
	pub rule: Arc<RuleMetadata>,
}
#[derive(Debug, Clone)]
pub struct SearchScope<'a> {
	pub rule: Arc<RuleMetadata>,
	pub location: &'a Location,
}
#[derive(Debug, Clone)]
pub struct ResourceScope {
	pub rule: Arc<RuleMetadata>,
	pub resource: Arc<Resource>,
}
#[derive(Debug, Clone)]
pub struct BatchScope<'a> {
	pub rule: Arc<RuleMetadata>,
	pub batch: &'a Batch,
}

#[derive(Debug, Clone)]
pub struct BuildScope {
	pub root: PathBuf,
}

/// The top-level context object, composed of the three distinct categories of information.
#[derive(Clone, Debug)]
pub struct ExecutionContext<'a> {
	pub services: &'a RunServices,
	pub scope: ExecutionScope<'a>,
	pub settings: &'a RunSettings,
}

impl<'a> ExecutionContext<'a> {
	pub fn with_scope(&self, scope: ExecutionScope<'a>) -> ExecutionContext {
		Self {
			services: self.services,
			scope,
			settings: self.settings,
		}
	}
}

// #[cfg(test)]
// pub struct ContextHarness {
// 	pub services: RunServices,
// 	pub settings: RunSettings,
// 	pub config: Config,
// 	pub rule: Rule,
// 	pub folder: Folder,
// 	pub resource: Resource,
// 	pub resources: Vec<Resource>,
// }

// #[cfg(test)]
// impl<'a> ContextHarness {
// 	/// Creates a new harness with default, dummy data.
// 	pub fn new(resource: Resource, resources: Vec<Resource>) -> Self {
// 		Self {
// 			services: RunServices::default(),
// 			config: Config::default(),
// 			settings: RunSettings {
// 				dry_run: true,
// 				no_parallel: true,
// 			},
// 			rule: Rule::default(),
// 			folder: Folder::default(),
// 			resource,
// 			resources,
// 		}
// 	}

// 	/// Returns a valid `ExecutionContext` with references to the harness's data.
// 	pub fn context(&'a self) -> ExecutionContext<'a> {
// 		let scope = ExecutionScope {
// 			config: &self.config,
// 			rule: &self.rule,
// 			folder: &self.folder,
// 			resource: &self.resource,
// 			resources: &self.resources,
// 		};
// 		ExecutionContext {
// 			services: &self.services,
// 			settings: &self.settings,
// 			scope,
// 		}
// 	}
// }
// Provide `Default` implementations for the final, compiled structs.
// These are only compiled for tests and allow for easy instantiation of dummy objects.
