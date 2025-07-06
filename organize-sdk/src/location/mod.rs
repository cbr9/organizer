use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
	context::{scope::ExecutionScope, ExecutionContext},
	error::Error,
	location::options::{Options, OptionsBuilder},
	plugins::storage::StorageProvider,
	templates::template::TemplateString,
};

pub mod options;

fn default_host() -> TemplateString {
	TemplateString("file".to_string())
}

#[derive(Debug, Serialize, PartialEq, Eq, Clone, Deserialize)]
pub struct LocationBuilder {
	pub path: TemplateString,
	#[serde(default = "default_host")]
	pub host: TemplateString,
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
	pub host: String,
	pub options: Options,
	pub mode: SearchMode,
	pub keep_structure: bool,
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
	pub async fn build(self, ctx: &ExecutionContext) -> Result<Location, Error> {
		let path_template = ctx.services.template_compiler.compile_template(&self.path)?;
		let path = PathBuf::from(path_template.render(ctx).await?);
		let host = ctx.services.template_compiler.compile_template(&self.host)?.render(ctx).await?;

		let scope = ExecutionScope::new_build_scope(&path);
		let ctx = ctx.with_scope(scope);

		Ok(Location {
			path: path.clone(),
			options: self.options.compile(&ctx, &host).await?,
			host,
			mode: self.mode,
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
