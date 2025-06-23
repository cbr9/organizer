use crate::{
	config::{actions::common::ConflictResolution, context::ExecutionContext},
	errors::{Error, ErrorContext},
	path::resolver::PathResolver,
	resource::Resource,
	templates::template::Template,
	utils::fs::ensure_parent_dir_exists,
};
use anyhow::Result;
use dashmap::DashSet;
use std::{future::Future, sync::Arc};

#[derive(Debug, Clone, Default)]
pub struct Locker {
	active_paths: Arc<DashSet<Resource>>,
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
		F: FnOnce(Resource) -> Fut,
		Fut: Future<Output = Result<T, Error>>,
	{
		let resolver = PathResolver::new(destination, strategy, extension, ctx);
		let Some(mut path) = resolver.resolve().await.map_err(|_| Error::PathResolution {
			template: destination.input.to_string(),
			context: ErrorContext::from_scope(&ctx.scope),
		})?
		else {
			return Ok(None);
		};

		let mut n = 1;
		let reserved = loop {
			if self.active_paths.contains(&path) {
				match strategy {
					ConflictResolution::Skip | ConflictResolution::Overwrite => return Ok(None),
					ConflictResolution::Rename => {
						let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
						let ext = path.extension().and_then(|s| s.to_str()).unwrap_or_default();
						let new_name = if ext.is_empty() {
							format!("{stem} ({n})")
						} else {
							format!("{stem} ({n}).{ext}")
						};
						path = path.with_file_name(new_name).into();
						n += 1;
						continue;
					}
				}
			}

			if path.try_exists(ctx).await? {
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
							format!("{stem} ({n})")
						} else {
							format!("{stem} ({n}).{ext}")
						};
						path = path.with_file_name(new_name).into();
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

		if let Some(target) = reserved {
			ensure_parent_dir_exists(&target).await.map_err(|e| Error::Io {
				source: e,
				path: ctx.scope.resource.clone(),
				target: Some(target.clone()),
				context: ErrorContext::from_scope(&ctx.scope),
			})?;

			let result = action(target.clone()).await?;

			self.active_paths.remove(&target);

			Ok(Some(result))
		} else {
			Ok(None)
		}
	}
}
