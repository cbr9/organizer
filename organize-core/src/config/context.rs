use dashmap::DashMap;
use std::{
	collections::HashMap,
	path::PathBuf,
	sync::{Arc, RwLock},
};

use lettre::{message::Mailbox, transport::smtp::authentication::Credentials};

use crate::{
	config::{folders::Folder, rule::Rule, Config},
	templates::TemplateEngine,
};

#[derive(Debug, Clone)]
pub struct RunServices {
	pub template_engine: TemplateEngine,
	pub credential_cache: Arc<RwLock<HashMap<Mailbox, Credentials>>>,
	pub content_cache: Arc<DashMap<PathBuf, Arc<String>>>,
}

impl Default for RunServices {
	fn default() -> Self {
		Self {
			template_engine: TemplateEngine::default(),
			credential_cache: Arc::new(RwLock::new(HashMap::new())),
			content_cache: Arc::new(DashMap::new()),
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
}

/// The top-level context object, composed of the three distinct categories of information.
#[derive(Clone)]
pub struct ExecutionContext<'a> {
	pub services: &'a RunServices,
	pub scope: ExecutionScope<'a>,
	pub settings: RunSettings,
}

#[cfg(test)]
pub struct ContextHarness {
	pub services: RunServices,
	pub config: Config,
	pub rule: Rule,
	pub folder: Folder,
}

#[cfg(test)]
impl ContextHarness {
	/// Creates a new harness with default, dummy data.
	pub fn new() -> Self {
		Self {
			services: RunServices::default(),
			config: Config::default(),
			rule: Rule::default(),
			folder: Folder::default(),
		}
	}

	/// Returns a valid `ExecutionContext` with references to the harness's data.
	pub fn context(&self) -> ExecutionContext {
		let scope = ExecutionScope {
			config: &self.config,
			rule: &self.rule,
			folder: &self.folder,
		};
		ExecutionContext {
			services: &self.services,
			scope,
			settings: RunSettings { dry_run: false },
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
			path: PathBuf::new(),
			options: Options::default(),
		}
	}
}
