use dashmap::DashMap;
use moka::future::{Cache, CacheBuilder};
use std::{any::Any, path::PathBuf, sync::Arc, time::Duration};

pub mod services;

use crate::{
	config::Config,
	context::services::{fs::manager::FileSystemManager, history::Journal},
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
#[derive(Debug, Clone)]
pub struct ExecutionScope<'a> {
	pub config: &'a Config,
	pub rule: &'a Rule,
	pub folder: &'a Folder,
	pub resource: Arc<Resource>,
	pub resources: Vec<Arc<Resource>>,
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
