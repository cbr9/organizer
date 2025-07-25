use std::{
	path::{Path, PathBuf},
	sync::Arc,
};

use crate::{context::ExecutionContext, error::Error, plugins::storage::StorageProvider, PROJECT_NAME};
use anyhow::Result;
use dirs;
use serde::{Deserialize, Serialize}; // Import the dirs crate
use uuid::Uuid; // Import Uuid for generating unique IDs // Import chrono for timestamps (already in Cargo.toml)

/// Determines the base directory for all backups.
/// This will be inside the platform-specific local data directory,
/// in a subdirectory named after the project, and then a "backups" folder.
fn get_backup_base_dir() -> Result<PathBuf, Error> {
	let project_name = &PROJECT_NAME;
	let base_dir = dirs::data_local_dir().expect("Could not determine platform-specific local data directory for backups.");
	let dir = base_dir.join(project_name).join("backups");
	Ok(dir)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Backup {
	path: PathBuf,
	backend: Arc<dyn StorageProvider>,
}

impl PartialEq for Backup {
	fn eq(&self, other: &Self) -> bool {
		self.path == other.path
	}
}

impl Eq for Backup {}

impl Backup {
	pub async fn new(ctx: &ExecutionContext) -> Result<Self, Error> {
		let dir = get_backup_base_dir()?;

		let backend = &ctx.scope.resource()?.backend;
		// Loop until a unique UUID is found for the backup filename
		let path = loop {
			let new_uuid = Uuid::new_v4().to_string();
			let proposed_path = dir.join(&new_uuid);

			if !backend.try_exists(&proposed_path, ctx).await? {
				break proposed_path;
			}
		};

		Ok(Self {
			path,
			backend: backend.clone(),
		})
	}

	pub async fn persist(&self, ctx: &ExecutionContext) -> Result<(), Error> {
		let resource = ctx.scope.resource()?;
		let from = resource.as_path();
		let to = self.path.as_path();

		// Attempt to hardlink
		if let Err(e) = self
			.backend
			.hardlink(from, to, ctx)
			.await
			.inspect(|_| tracing::debug!(backup_path = %self.path.display(), file = %from.display(), "Backup complete"))
		{
			// If hardlink fails, try copy and delete
			eprintln!("Hardlink failed: {e}. Falling back to copy and delete.");
			self.backend.copy(from, to, ctx).await?;
			tracing::debug!(backup_path = %self.path.display(), file = %from.display(), "Backup complete");
		}

		Ok(())
	}

	pub async fn restore(&self, to: &Path, ctx: &ExecutionContext) -> Result<(), Error> {
		let from = self.path.as_path();
		self.backend.rename(from, to, ctx).await?;
		tracing::debug!(backup_path = %self.path.display(), file = %to.display(), "Backup restored");
		Ok(())
	}
}
