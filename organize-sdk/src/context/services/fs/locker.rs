use crate::{
	context::{services::fs::manager::Destination, ExecutionContext},
	engine::ConflictResolution,
	error::Error,
};
use anyhow::Result;
use dashmap::DashSet;
use std::{ops::Deref, path::PathBuf, sync::Arc};

#[derive(Debug)]
pub struct LockGuard {
	path: PathBuf,
	active_paths: Arc<DashSet<PathBuf>>,
}

impl Deref for LockGuard {
	type Target = PathBuf;

	fn deref(&self) -> &Self::Target {
		&self.path
	}
}

impl Drop for LockGuard {
	fn drop(&mut self) {
		self.active_paths.remove(&self.path);
	}
}

#[derive(Debug, Clone, Default)]
pub struct Locker {
	active_paths: Arc<DashSet<PathBuf>>,
}

impl Locker {
	pub async fn lock_destination(&self, ctx: &ExecutionContext, destination: &Destination) -> Result<Option<LockGuard>, Error> {
		let mut path = destination.resolve(ctx).await?;
		let mut n = 1;

		let reserved = loop {
			if self.active_paths.contains(&path) {
				match destination.resolution_strategy {
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

			let res = ctx
				.services
				.fs
				.get_or_init_resource(path.to_path_buf(), None, &destination.host)
				.await?;
			if ctx.services.fs.try_exists(&res, ctx).await? {
				match destination.resolution_strategy {
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

		if let Some(path) = reserved {
			ctx.services.fs.ensure_parent_dir_exists(&path).await?;
			let guard = LockGuard {
				path,
				active_paths: self.active_paths.clone(),
			};
			Ok(Some(guard))
		} else {
			Ok(None)
		}
	}
}
