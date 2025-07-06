use crate::{
	context::{services::fs::manager::Destination, ExecutionContext},
	engine::ConflictResolution,
	error::Error,
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
		ctx: &ExecutionContext,
		destination: &Destination,
		strategy: &ConflictResolution,
		action: F,
	) -> Result<Option<T>, Error>
	where
		F: FnOnce(PathBuf) -> Fut,
		Fut: Future<Output = Result<T, Error>>,
	{
		let mut path = destination.resolve(ctx).await?;
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
						path = path.with_file_name(new_name);
						n += 1;
						continue;
					}
				}
			}

			let exists = if let Some(res) = ctx.services.fs.resources.get(&path).await {
				res.try_exists(ctx).await?
			} else {
				tokio::fs::try_exists(&path).await?
			};

			if exists {
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
						path = path.with_file_name(new_name);
						n += 1;
						continue;
					}
				}
			}

			if !self.active_paths.insert(path.to_path_buf()) {
				continue;
			}
			break Some(path);
		};

		if let Some(target) = reserved {
			ctx.services.fs.ensure_parent_dir_exists(&target).await?;
			let result = action(target.clone()).await?;

			self.active_paths.remove(&target.to_path_buf());

			Ok(Some(result))
		} else {
			Ok(None)
		}
	}
}
