use std::{
	path::PathBuf,
	sync::Arc,
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
	context::{scope::ExecutionScope, services::fs::manager::parse_uri, ExecutionContext},
	error::Error,
	location::options::{Options, OptionsBuilder},
	plugins::storage::StorageProvider,
	templates::template::TemplateString,
};

pub mod options;

#[derive(Debug, Serialize, PartialEq, Eq, Clone, Deserialize)]
pub struct LocationBuilder {
	pub path: TemplateString,
	#[serde(flatten)]
	pub options: OptionsBuilder,
	#[serde(default)]
	pub mode: SearchMode,
	#[serde(default = "defaults::keep_structure")]
	pub keep_structure: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Location {
	pub path: PathBuf,
	pub options: Options,
	pub mode: SearchMode,
	pub keep_structure: bool,
	pub backend: Arc<dyn StorageProvider>,
}

impl Eq for Location {}

impl PartialEq for Location {
	fn eq(&self, other: &Self) -> bool {
		self.path == other.path && self.options == other.options && self.mode == other.mode && self.keep_structure == other.keep_structure
	}
}

impl Location {
	pub fn new_local() -> Arc<dyn StorageProvider> {
		let value = serde_json::json!({
			"type": "local"
		});
		let backend: Box<dyn StorageProvider> = serde_json::from_value(value).unwrap();
		Arc::from(backend)
	}
}

impl LocationBuilder {
	pub async fn build(self, ctx: &ExecutionContext<'_>) -> Result<Location, Error> {
		let path_template = ctx.services.compiler.compile_template(&self.path)?;
		let uri = path_template.render(ctx).await?;
		let (host, path) = parse_uri(&uri)?;
		let path = PathBuf::from(path);

		let ctx = &ExecutionContext {
			services: ctx.services,
			scope: ExecutionScope::new_build_scope(&path),
			settings: ctx.settings,
		};

		Ok(Location {
			path,
			options: self.options.compile(ctx).await?,
			mode: self.mode,
			backend: ctx.services.fs.backends.get(&host).unwrap().clone(), // The direct Arc clone to the provider
			keep_structure: self.keep_structure,
		})
	}
}

mod defaults {
	pub(super) fn keep_structure() -> bool {
		true
	}
}
/// The final, compiled `Folder` object, ready for execution.

#[derive(Debug, Deserialize, Serialize, Default, PartialEq, Eq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum SearchMode {
	Replace,
	#[default]
	Append,
}

impl SearchMode {
	/// Returns `true` if the search mode is [`Replace`].
	///
	/// [`Replace`]: SearchMode::Replace
	#[must_use]
	pub fn is_replace(&self) -> bool {
		matches!(self, Self::Replace)
	}

	/// Returns `true` if the search mode is [`Append`].
	///
	/// [`Append`]: SearchMode::Append
	#[must_use]
	pub fn is_append(&self) -> bool {
		matches!(self, Self::Append)
	}
}
