// organize-core/src/utils/backup.rs

use crate::{
	config::context::ExecutionContext,
	errors::{Error, ErrorContext},
};
use crate::{utils, PROJECT_NAME}; // Import PROJECT_NAME from lib.rs
use anyhow::Result;
use dirs;
use serde::{Deserialize, Serialize}; // Import the dirs crate
use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid; // Import Uuid for generating unique IDs // Import chrono for timestamps (already in Cargo.toml)

/// Determines the base directory for all backups.
/// This will be inside the platform-specific local data directory,
/// in a subdirectory named after the project, and then a "backups" folder.
fn get_backup_base_dir() -> PathBuf {
	let base_dir = dirs::data_local_dir().expect("Could not determine platform-specific local data directory for backups.");
	base_dir.join(PROJECT_NAME).join("backups")
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Backup {
	pub id: String,
	pub path: PathBuf,
	pub original: PathBuf,
}

impl Backup {
	pub async fn new(original: impl AsRef<Path>, ctx: &ExecutionContext<'_>) -> Result<Self, Error> {
		let backup_base_dir = get_backup_base_dir();

		// Loop until a unique UUID is found for the backup filename
		let (id, path) = loop {
			let new_uuid = Uuid::new_v4().to_string();
			let proposed_path = backup_base_dir.join(&new_uuid);

			if !utils::fs::try_exists(&proposed_path, ctx).await? {
				break (new_uuid, proposed_path);
			}
		};
		Ok(Self {
			id,
			path,
			original: original.as_ref().to_path_buf(),
		})
	}

	pub async fn persist(&self, ctx: &ExecutionContext<'_>) -> Result<(), Error> {
		let backup_base_dir = get_backup_base_dir();
		fs::create_dir_all(&backup_base_dir).await.map_err(|e| Error::Io {
			source: e,
			path: backup_base_dir.to_path_buf(),
			target: None,
			context: ErrorContext::from_scope(&ctx.scope),
		})?;

		// Perform the copy operation
		fs::copy(&self.original, &self.path).await.map_err(|e| Error::Io {
			source: e,
			path: self.original.clone(),
			target: Some(self.path.clone()), // Use the cloned backup_path for target
			context: ErrorContext::from_scope(&ctx.scope),
		})?;

		Ok(())
	}
}
