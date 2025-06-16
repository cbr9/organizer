use std::{
	collections::HashMap,
	sync::{Arc, RwLock},
};

use lettre::{message::Mailbox, transport::smtp::authentication::Credentials};

use crate::{
	config::{folders::Folder, rule::Rule, Config},
	templates::TemplateEngine,
};

/// A container for all contextual information required for a single operation.
/// It is generic over a lifetime `'a` to hold references to the configuration tree.
#[derive(Clone)]
pub struct Context<'a> {
	pub config: &'a Config,
	pub rule: &'a Rule,
	pub folder: &'a Folder,
	pub template_engine: &'a TemplateEngine,
	pub dry_run: bool,
	pub email_credentials: Arc<RwLock<HashMap<Mailbox, Credentials>>>,
}

/// A test harness to simplify the creation of a `Context`.
/// It owns all the necessary data, allowing it to return a context
/// with valid references.
#[cfg(test)]
pub struct ContextHarness {
	pub config: Config,
	pub rule: Rule,
	pub folder: Folder,
	pub dry_run: bool,
	pub template_engine: TemplateEngine,
	pub email_credentials: Arc<RwLock<HashMap<Mailbox, Credentials>>>,
}

#[cfg(test)]
impl ContextHarness {
	/// Creates a new harness with default, dummy data.
	pub fn new() -> Self {
		Self {
			config: Config::default(),
			rule: Rule::default(),
			folder: Folder::default(),
			template_engine: TemplateEngine::default(),
			dry_run: false,
			email_credentials: Arc::new(RwLock::new(HashMap::new())),
		}
	}

	/// Returns a valid `Context` with references to the harness's data.
	pub fn context(&self) -> Context {
		Context {
			config: &self.config,
			rule: &self.rule,
			folder: &self.folder,
			template_engine: &self.template_engine,
			dry_run: self.dry_run,
			email_credentials: self.email_credentials.clone(),
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
		use crate::templates::TemplateEngine;

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
