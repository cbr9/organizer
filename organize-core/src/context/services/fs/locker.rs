use crate::{
	context::{services::fs::manager::Destination, ExecutionContext},
	engine::ConflictResolution,
	errors::{Error, ErrorContext},
	resource::Resource,
	stdx::path::PathExt,
};
use anyhow::Result;
use dashmap::DashSet;
use std::{future::Future, path::PathBuf, sync::Arc};

#[derive(Debug, Clone, Default)]
pub struct Locker {
	active_paths: Arc<DashSet<PathBuf>>,
}

impl Locker {
	pub async fn with_locked_destination<F, Fut, T>(
		&self,
		ctx: &ExecutionContext<'_>,
		destination: &Destination,
		strategy: &ConflictResolution,
		action: F,
	) -> Result<Option<T>, Error>
	where
		F: FnOnce(Arc<Resource>) -> Fut,
		Fut: Future<Output = Result<T, Error>>,
	{
		let mut path = destination.get_final_path(ctx).await?.as_resource(ctx).await;
		let mut n = 1;

		dbg!(&self.active_paths);
		dbg!(&path.path.display());
		dbg!(&strategy);
		let reserved = loop {
			if self.active_paths.contains(&path.to_path_buf()) {
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
						path = path.with_file_name(new_name).as_resource(ctx).await;
						n += 1;
						continue;
					}
				}
			}

			if path.try_exists(ctx).await? {
				match strategy {
					ConflictResolution::Skip => return Ok(None),
					ConflictResolution::Overwrite => {
						if !self.active_paths.insert(path.to_path_buf()) {
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
						path = path.with_file_name(new_name).as_resource(ctx).await;
						n += 1;
						continue;
					}
				}
			}

			dbg!(&self.active_paths);
			if !self.active_paths.insert(path.to_path_buf()) {
				println!("MMM");
				continue;
			}
			println!("{}", path.path.display());
			break Some(path);
		};

		dbg!(&reserved);

		if let Some(target) = reserved {
			ctx.services.fs.ensure_parent_dir_exists(&target).await.map_err(|e| Error::Io {
				source: e,
				path: ctx.scope.resource.clone(),
				target: Some(target.clone()),
				context: ErrorContext::from_scope(&ctx.scope),
			})?;

			let result = action(target.clone()).await?;

			self.active_paths.remove(&target.to_path_buf());

			Ok(Some(result))
		} else {
			Ok(None)
		}
	}
}
