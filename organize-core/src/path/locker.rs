use crate::{
	config::{actions::common::ConflictResolution, context::ExecutionContext},
	errors::{Error, ErrorContext},
	path::resolver::PathResolver,
	templates::template::Template,
};
use anyhow::Result;
use dashmap::{DashMap, DashSet};
use std::{
	future::Future,
	path::{Path, PathBuf},
	sync::Arc,
};
use tokio::sync::Mutex;

pub async fn ensure_parent_dir_exists(path: &Path) -> std::io::Result<()> {
	if let Some(parent) = path.parent() {
		if !tokio::fs::try_exists(parent).await.unwrap_or(false) {
			tokio::fs::create_dir_all(parent).await?;
		}
	}
	Ok(())
}

#[derive(Debug, Clone, Default)]
pub struct Locker {
	active_paths: Arc<DashSet<PathBuf>>,
}

impl Locker {
	pub async fn with_locked_destination<F, Fut, T>(
		&self,
		ctx: &ExecutionContext<'_>,
		destination: &Template,
		strategy: &ConflictResolution,
		extension: bool,
		action: F,
	) -> Result<Option<T>, Error>
	where
		F: FnOnce(PathBuf) -> Fut,
		Fut: Future<Output = Result<T, Error>>,
	{
		// ... (resolver and path initialization logic remains the same) ...
		let resolver = PathResolver::new(destination, strategy, extension, ctx);
		let Some(mut path) = resolver.resolve().await.map_err(|_| Error::PathResolution {
			template: destination.input.to_string(),
			context: ErrorContext::from_scope(&ctx.scope),
		})?
		else {
			return Ok(None);
		};

		let mut n = 1;
		let reserved_path = loop {
			if self.active_paths.contains(&path) {
				// ... (conflict resolution logic remains the same) ...
				match strategy {
					ConflictResolution::Skip | ConflictResolution::Overwrite => return Ok(None),
					ConflictResolution::Rename => {
						let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
						let ext = path.extension().and_then(|s| s.to_str()).unwrap_or_default();
						let new_name = if ext.is_empty() {
							format!("{} ({})", stem, n)
						} else {
							format!("{} ({}).{}", stem, n, ext)
						};
						path.set_file_name(new_name);
						n += 1;
						continue;
					}
				}
			}

			let exists = if ctx.settings.dry_run {
				ctx.services.blackboard.simulated_paths.contains(&path)
			} else {
				tokio::fs::try_exists(&path).await.unwrap_or(false)
			};

			if exists {
				match strategy {
					ConflictResolution::Skip => return Ok(None),
					ConflictResolution::Overwrite => {
						if !self.active_paths.insert(path.clone()) {
							return Ok(None);
						}
						break Some(path);
					}
					ConflictResolution::Rename => {
						let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
						let ext = path.extension().and_then(|s| s.to_str()).unwrap_or_default();
						let new_name = if ext.is_empty() {
							format!("{} ({})", stem, n)
						} else {
							format!("{} ({}).{}", stem, n, ext)
						};
						path.set_file_name(new_name);
						n += 1;
						continue;
					}
				}
			}

			if !self.active_paths.insert(path.clone()) {
				continue;
			}
			break Some(path);
		};
		// ... (action execution and release logic remains the same) ...
		if let Some(target_path) = reserved_path {
			ensure_parent_dir_exists(&target_path).await.map_err(|e| Error::Io {
				source: e,
				path: ctx.scope.resource.path().to_path_buf(),
				target: Some(target_path.clone()),
				context: ErrorContext::from_scope(&ctx.scope),
			})?;

			let result = action(target_path.clone()).await?;

			self.active_paths.remove(&target_path);

			Ok(Some(result))
		} else {
			Ok(None)
		}
	}
}
