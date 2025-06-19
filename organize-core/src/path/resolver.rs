use crate::{
	config::{actions::common::ConflictResolution, context::ExecutionContext},
	templates::{template::Template, Context},
};
use anyhow::Result;
use std::path::{PathBuf, MAIN_SEPARATOR};

/// Resolves a template into a final, conflict-resolved path.
/// It does not perform locking or directory creation.
pub struct PathResolver<'a> {
	pub template: &'a Template,
	pub strategy: &'a ConflictResolution,
	pub extension: bool,
	pub ctx: &'a ExecutionContext<'a>,
}

impl<'a> PathResolver<'a> {
	pub fn new(template: &'a Template, strategy: &'a ConflictResolution, extension: bool, ctx: &'a ExecutionContext) -> Self {
		Self {
			template,
			strategy,
			extension,
			ctx,
		}
	}

	pub async fn resolve(&self) -> Result<Option<PathBuf>> {
		let context = Context::new(self.ctx);
		let templater = &self.ctx.services.templater;
		let Some(mut path) = templater.render(self.template, &context)?.map(PathBuf::from) else {
			return Ok(None);
		};

		if path.is_dir() || path.to_string_lossy().ends_with(MAIN_SEPARATOR) || path.to_string_lossy().ends_with('/') {
			if self.extension {
				if let Some(filename) = self.ctx.scope.resource.path().file_name() {
					path.push(filename);
				} else {
					return Ok(None); // Cannot move a file that has no name (e.g., "/")
				}
			} else if let Some(stem) = self.ctx.scope.resource.path().file_stem() {
				path.push(stem);
			} else {
				return Ok(None);
			}
		}

		if tokio::fs::try_exists(&path).await? {
			// This helper would also need to be async
			return self.strategy.resolve(&path).await;
		}
		Ok(Some(path))
	}
}
