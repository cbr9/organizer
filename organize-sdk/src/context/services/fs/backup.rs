use std::path::PathBuf;

use crate::{context::ExecutionContext, error::Error};
use anyhow::Result;
use dirs;
use serde::{Deserialize, Serialize}; // Import the dirs crate
use tokio::fs;
use uuid::Uuid; // Import Uuid for generating unique IDs // Import chrono for timestamps (already in Cargo.toml)

/// Determines the base directory for all backups.
/// This will be inside the platform-specific local data directory,
/// in a subdirectory named after the project, and then a "backups" folder.
fn get_backup_base_dir(_ctx: &ExecutionContext<'_>) -> Result<PathBuf, Error> {
	let project_name = env!("CARGO_PKG_NAME");
	let base_dir = dirs::data_local_dir().expect("Could not determine platform-specific local data directory for backups.");
	let dir = base_dir.join(project_name).join("backups");
	Ok(dir)
}

#[derive(Default, Clone, Deserialize, Serialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum BackupLocation {
	#[default]
	System,
	Root,
	Custom(PathBuf),
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Backup(pub PathBuf);

impl std::ops::Deref for Backup {
	type Target = PathBuf;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl Backup {
	pub async fn new(ctx: &ExecutionContext<'_>) -> Result<Self, Error> {
		let dir = get_backup_base_dir(ctx)?;

		// Loop until a unique UUID is found for the backup filename
		let path = loop {
			let new_uuid = Uuid::new_v4().to_string();
			let proposed_path = dir.join(&new_uuid);

			if !tokio::fs::try_exists(&proposed_path).await? {
				break proposed_path;
			}
		};
		Ok(Self(path))
	}

	pub async fn persist(&self, ctx: &ExecutionContext<'_>) -> Result<(), Error> {
		let parent = self.0.parent().unwrap();
		fs::create_dir_all(parent).await?;
		let source = ctx.scope.resource()?;

		match fs::hard_link(source.as_path(), self.0.as_path()).await {
			Ok(()) => {
				tracing::debug!("Created hard link backup for {}", source.as_path().display());
				Ok(())
			}
			Err(e) if e.raw_os_error() == Some(libc::EXDEV) || e.kind() == std::io::ErrorKind::CrossesDevices => {
				tracing::warn!(
					"Backup for {} is on a different filesystem. Falling back to a full copy.",
					ctx.scope.resource()?.as_path().display()
				);
				fs::copy(ctx.scope.resource()?.as_path(), self.0.as_path()).await?;
				Ok(())
			}
			Err(e) => Err(Error::Io(e)),
		}
	}
}
