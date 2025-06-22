// organize-core/src/utils/backup.rs

use crate::{
	config::context::ExecutionContext,
	errors::{Error, ErrorContext},
	resource::Resource,
};
use crate::PROJECT_NAME; // Import PROJECT_NAME from lib.rs
use anyhow::Result;
use dirs;
use serde::{Deserialize, Serialize}; // Import the dirs crate
use tokio::fs;
use uuid::Uuid; // Import Uuid for generating unique IDs // Import chrono for timestamps (already in Cargo.toml)

/// Determines the base directory for all backups.
/// This will be inside the platform-specific local data directory,
/// in a subdirectory named after the project, and then a "backups" folder.
fn get_backup_base_dir() -> Resource {
	let base_dir = dirs::data_local_dir().expect("Could not determine platform-specific local data directory for backups.");
	base_dir.join(PROJECT_NAME).join("backups").into()
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Backup(pub Resource);

impl std::ops::Deref for Backup {
	type Target = Resource;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl Backup {
	pub async fn new(ctx: &ExecutionContext<'_>) -> Result<Self, Error> {
		let backup_base_dir = get_backup_base_dir();

		// Loop until a unique UUID is found for the backup filename
		let path = loop {
			let new_uuid = Uuid::new_v4().to_string();
			let proposed_path: Resource = backup_base_dir.join(&new_uuid).into();

			if !proposed_path.try_exists(ctx).await? {
				break proposed_path;
			}
		};
		Ok(Self(path))
	}

	pub async fn persist(&self, original: Resource, ctx: &ExecutionContext<'_>) -> Result<(), Error> {
		if ctx.scope.folder.settings.backup {
			let backup_base_dir = get_backup_base_dir();
			fs::create_dir_all(&backup_base_dir).await.map_err(|e| Error::Io {
				source: e,
				path: backup_base_dir,
				target: None,
				context: ErrorContext::from_scope(&ctx.scope),
			})?;

			// Perform the copy operation
			fs::copy(&original, &self.0).await.map_err(|e| Error::Io {
				source: e,
				path: original.clone(),
				target: Some(self.0.clone()), // Use the cloned backup_path for target
				context: ErrorContext::from_scope(&ctx.scope),
			})?;
		}

		Ok(())
	}
}
