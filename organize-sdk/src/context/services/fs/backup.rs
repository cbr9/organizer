use std::{path::PathBuf, sync::Arc};

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
	host: String,
	backend: Arc<dyn StorageProvider>,
}

impl PartialEq for Backup {
	fn eq(&self, other: &Self) -> bool {
		self.path == other.path && self.host == other.host
	}
}

impl Backup {
	pub async fn new(host: &str, ctx: &ExecutionContext) -> Result<Self, Error> {
		let dir = get_backup_base_dir()?;

		// Loop until a unique UUID is found for the backup filename
		let path = loop {
			let new_uuid = Uuid::new_v4().to_string();
			let proposed_path = dir.join(&new_uuid);

			if !tokio::fs::try_exists(&proposed_path).await? {
				break proposed_path;
			}
		};

		let backend = ctx.services.fs.get_provider(host)?;
		Ok(Self {
			path,
			host: host.to_string(),
			backend,
		})
	}

	pub async fn persist(&self, ctx: &ExecutionContext) -> Result<(), Error> {
		let resource = ctx.scope.resource()?;
		let from = resource.as_path();
		let to = self.path.as_path();

		// Attempt to hardlink
		if let Err(e) = self
			.backend
			.hardlink(from, to)
			.await
			.inspect(|_| tracing::debug!(backup_path = %self.path.display(), file = %from.display(), "Backup complete"))
		{
			// If hardlink fails, try copy and delete
			eprintln!("Hardlink failed: {}. Falling back to copy and delete.", e);
			self.backend.copy(from, to).await?;
			tracing::debug!(backup_path = %self.path.display(), file = %from.display(), "Backup complete");
		}

		Ok(())
	}
}
