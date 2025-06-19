use dashmap::DashMap;
use std::{
	any::Any,
	collections::HashMap,
	path::PathBuf,
	sync::{Arc, RwLock},
};

use lettre::{message::Mailbox, transport::smtp::authentication::Credentials};

use crate::{
	config::{folders::Folder, rule::Rule, Config},
	templates::Templater,
};

#[derive(Debug, Clone)]
pub struct RunServices {
	pub templater: Templater,
	pub blackboard: Blackboard,
}

#[derive(Debug, Default, Clone)]
pub struct Blackboard {
	pub credentials: Arc<RwLock<HashMap<Mailbox, Credentials>>>,
	pub content: Arc<DashMap<PathBuf, Arc<String>>>,
	pub scratchpad: Arc<DashMap<String, Box<dyn Any + Send + Sync>>>,
}

impl Default for RunServices {
	fn default() -> Self {
		Self {
			templater: Templater::default(),
			blackboard: Blackboard::default(),
		}
	}
}

/// A container for run-wide operational settings.
#[derive(Debug, Clone, Copy)]
pub struct RunSettings {
	pub dry_run: bool,
	pub no_parallel: bool,
}

/// A read-only "view" into the current position in the configuration tree.
#[derive(Debug, Clone)]
pub struct ExecutionScope<'a> {
	pub config: &'a Config,
	pub rule: &'a Rule,
	pub folder: &'a Folder,
}

/// The top-level context object, composed of the three distinct categories of information.
#[derive(Clone, Debug)]
pub struct ExecutionContext<'a> {
	pub services: &'a RunServices,
	pub scope: ExecutionScope<'a>,
	pub settings: &'a RunSettings,
}

#[cfg(test)]
pub struct ContextHarness {
	pub services: RunServices,
	pub settings: RunSettings,
	pub config: Config,
	pub rule: Rule,
	pub folder: Folder,
}

#[cfg(test)]
impl<'a> ContextHarness {
	/// Creates a new harness with default, dummy data.
	pub fn new() -> Self {
		Self {
			services: RunServices::default(),
			config: Config::default(),
			settings: RunSettings {
				dry_run: true,
				no_parallel: true,
			},
			rule: Rule::default(),
			folder: Folder::default(),
		}
	}

	/// Returns a valid `ExecutionContext` with references to the harness's data.
	pub fn context(&'a self) -> ExecutionContext<'a> {
		let scope = ExecutionScope {
			config: &self.config,
			rule: &self.rule,
			folder: &self.folder,
		};
		ExecutionContext {
			services: &self.services,
			settings: &self.settings,
			scope,
		}
	}
}
// Provide `Default` implementations for the final, compiled structs.
// These are only compiled for tests and allow for easy instantiation of dummy objects.
#[cfg(test)]
impl Default for Config {
	fn default() -> Self {
		use std::path::PathBuf;

		Self {
			rules: Vec::new(),
			path: PathBuf::new(),
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
		}
	}
}

#[cfg(test)]
impl Default for Folder {
	fn default() -> Self {
		use std::path::PathBuf;

		use crate::config::options::Options;

		Self {
			index: 0,
			path: PathBuf::new(),
			options: Options::default(),
		}
	}
}
