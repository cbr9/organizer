use anyhow::Result;
use dashmap::DashMap;
use moka::future::{Cache, CacheBuilder};
use std::{any::Any, path::PathBuf, sync::Arc, time::Duration};

pub mod services;

use crate::{
	config::Config,
	context::services::{fs::manager::FileSystemManager, history::Journal},
	errors::Error,
	folder::Folder,
	resource::Resource,
	rule::Rule,
	templates::engine::Templater,
};

#[derive(Debug, Clone)]
pub struct RunServices {
	pub templater: Templater,
	pub blackboard: Blackboard,
	pub fs: FileSystemManager,
	pub journal: Arc<Journal>,
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct VariableCacheKey {
	pub rule_index: usize,
	pub variable: String,
	pub resource: Resource,
}

#[derive(Debug, Clone)]
pub struct Blackboard {
	pub scratchpad: Arc<DashMap<String, Box<dyn Any + Send + Sync>>>,
	pub resources: Cache<PathBuf, Resource>,
}

impl Default for Blackboard {
	fn default() -> Self {
		Self {
			scratchpad: Arc::new(DashMap::new()),
			resources: CacheBuilder::new(1_000_000)
				.time_to_live(Duration::new(60 * 60 * 24, 0)) // ONE DAY
				.name("cached_resources")
				.build(),
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
	Config(ConfigScope<'a>),
	Rule(RuleScope<'a>),
	Folder(FolderScope<'a>),
	Resource(ResourceScope<'a>),
	Batch(BatchScope<'a>),
}

impl<'a> ExecutionScope<'a> {
	pub fn new_config_scope(config: &'a Config) -> ExecutionScope<'a> {
		ExecutionScope::Config(ConfigScope { config })
	}

	pub fn new_rule_scope(config: &'a Config, rule: &'a Rule) -> ExecutionScope<'a> {
		ExecutionScope::Rule(RuleScope { config, rule })
	}

	pub fn new_folder_scope(config: &'a Config, rule: &'a Rule, folder: &'a Folder) -> ExecutionScope<'a> {
		ExecutionScope::Folder(FolderScope { config, rule, folder })
	}

	pub fn new_resource_scope(config: &'a Config, rule: &'a Rule, folder: &'a Folder, resource: Arc<Resource>) -> ExecutionScope<'a> {
		ExecutionScope::Resource(ResourceScope {
			config,
			rule,
			folder,
			resource,
		})
	}

	pub fn new_batch_scope(config: &'a Config, rule: &'a Rule, folder: &'a Folder, batch: Vec<Arc<Resource>>) -> ExecutionScope<'a> {
		ExecutionScope::Batch(BatchScope { config, rule, folder, batch })
	}

	pub fn config(&self) -> Result<&'a Config, Error> {
		match self {
			ExecutionScope::Config(scope) => Ok(scope.config),
			ExecutionScope::Rule(scope) => Ok(scope.config),
			ExecutionScope::Folder(scope) => Ok(scope.config),
			ExecutionScope::Resource(scope) => Ok(scope.config),
			ExecutionScope::Batch(scope) => Ok(scope.config),
		}
	}

	pub fn rule(&self) -> Result<&'a Rule, Error> {
		match self {
			ExecutionScope::Config(_scope) => Err(Error::ScopeError("rule".into())),
			ExecutionScope::Rule(scope) => Ok(scope.rule),
			ExecutionScope::Folder(scope) => Ok(scope.rule),
			ExecutionScope::Resource(scope) => Ok(scope.rule),
			ExecutionScope::Batch(scope) => Ok(scope.rule),
		}
	}

	pub fn folder(&self) -> Result<&'a Folder, Error> {
		match self {
			ExecutionScope::Config(_scope) => Err(Error::ScopeError("folder".into())),
			ExecutionScope::Rule(_scope) => Err(Error::ScopeError("folder".into())),
			ExecutionScope::Folder(scope) => Ok(scope.folder),
			ExecutionScope::Resource(scope) => Ok(scope.folder),
			ExecutionScope::Batch(scope) => Ok(scope.folder),
		}
	}

	pub fn resource(&self) -> Result<Arc<Resource>, Error> {
		match self {
			ExecutionScope::Config(_scope) => Err(Error::ScopeError("resource".into())),
			ExecutionScope::Rule(_scope) => Err(Error::ScopeError("resource".into())),
			ExecutionScope::Folder(_scope) => Err(Error::ScopeError("resource".into())),
			ExecutionScope::Resource(scope) => Ok(scope.resource.clone()),
			ExecutionScope::Batch(_scope) => Err(Error::ScopeError("resource".into())),
		}
	}

	pub fn batch(&self) -> Result<Vec<Arc<Resource>>, Error> {
		match self {
			ExecutionScope::Config(_scope) => Err(Error::ScopeError("batch".into())),
			ExecutionScope::Rule(_scope) => Err(Error::ScopeError("batch".into())),
			ExecutionScope::Folder(_scope) => Err(Error::ScopeError("batch".into())),
			ExecutionScope::Resource(_scope) => Err(Error::ScopeError("batch".into())),
			ExecutionScope::Batch(scope) => Ok(scope.batch.clone()),
		}
	}
}

#[derive(Debug, Clone)]
pub struct ConfigScope<'a> {
	pub config: &'a Config,
}
#[derive(Debug, Clone)]
pub struct RuleScope<'a> {
	pub config: &'a Config,
	pub rule: &'a Rule,
}
#[derive(Debug, Clone)]
pub struct FolderScope<'a> {
	pub config: &'a Config,
	pub rule: &'a Rule,
	pub folder: &'a Folder,
}
#[derive(Debug, Clone)]
pub struct ResourceScope<'a> {
	pub config: &'a Config,
	pub rule: &'a Rule,
	pub folder: &'a Folder,
	pub resource: Arc<Resource>,
}
#[derive(Debug, Clone)]
pub struct BatchScope<'a> {
	pub config: &'a Config,
	pub rule: &'a Rule,
	pub folder: &'a Folder,
	pub batch: Vec<Arc<Resource>>,
}

/// The top-level context object, composed of the three distinct categories of information.
#[derive(Clone, Debug)]
pub struct ExecutionContext<'a> {
	pub services: &'a RunServices,
	pub scope: ExecutionScope<'a>,
	pub settings: &'a RunSettings,
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
#[cfg(test)]
impl Default for Config {
	fn default() -> Self {
		use std::path::PathBuf;

		Self {
			rules: Vec::new(),
			path: PathBuf::new(),
			variables: vec![],
		}
	}
}

#[cfg(test)]
impl Default for Rule {
	fn default() -> Self {
		Self {
			id: None,
			index: 0,
			tags: Default::default(),
			actions: Default::default(),
			filters: Default::default(),
			folders: Default::default(),
			variables: Default::default(),
		}
	}
}

#[cfg(test)]
impl Default for Folder {
	fn default() -> Self {
		use std::path::PathBuf;

		use crate::options::Options;

		Self {
			index: 0,
			path: PathBuf::new(),
			settings: Options::default(),
		}
	}
}
