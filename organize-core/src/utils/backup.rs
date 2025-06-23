// organize-core/src/utils/backup.rs

use crate::PROJECT_NAME; // Import PROJECT_NAME from lib.rs
use crate::{
	config::context::ExecutionContext,
	errors::{Error, ErrorContext},
	resource::Resource,
};
use anyhow::Result;
use dirs;
use serde::{Deserialize, Serialize}; // Import the dirs crate
use tokio::fs;
use uuid::Uuid; // Import Uuid for generating unique IDs // Import chrono for timestamps (already in Cargo.toml)

/// Determines the base directory for all backups.
/// This will be inside the platform-specific local data directory,
/// in a subdirectory named after the project, and then a "backups" folder.
fn get_backup_base_dir(ctx: &ExecutionContext) -> Resource {
	match &ctx.scope.folder.settings.backup_location {
		BackupLocation::System => {
			let base_dir = dirs::data_local_dir().expect("Could not determine platform-specific local data directory for backups.");
			base_dir.join(PROJECT_NAME).join("backups").into()
		}
		BackupLocation::Root => ctx.scope.folder.path.join(".organize").join("backups").into(),
		BackupLocation::Custom(path) => path.clone(),
	}
}

#[derive(Default, Clone, Deserialize, Serialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum BackupLocation {
	#[default]
	System,
	Root,
	Custom(Resource),
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
		let dir = get_backup_base_dir(ctx);

		// Loop until a unique UUID is found for the backup filename
		let path = loop {
			let new_uuid = Uuid::new_v4().to_string();
			let proposed_path: Resource = dir.join(&new_uuid).into();

			if !proposed_path.try_exists(ctx).await? {
				break proposed_path;
			}
		};
		Ok(Self(path))
	}

	pub async fn persist(&self, ctx: &ExecutionContext<'_>) -> Result<(), Error> {
		fs::create_dir_all(&self.0.parent().unwrap()).await.map_err(|e| Error::Io {
			source: e,
			path: self.0.parent().unwrap().into(),
			target: None,
			context: ErrorContext::from_scope(&ctx.scope),
		})?;

		match fs::hard_link(&ctx.scope.resource, &self.0).await {
			Ok(()) => {
				tracing::debug!("Created hard link backup for {}", ctx.scope.resource.display());
				Ok(())
			}
			Err(e) if e.raw_os_error() == Some(libc::EXDEV) || e.kind() == std::io::ErrorKind::CrossesDevices => {
				tracing::warn!(
					"Backup for {} is on a different filesystem. Falling back to a full copy.",
					ctx.scope.resource.display()
				);
				fs::copy(&ctx.scope.resource, &self.0).await.map_err(|e| Error::Io {
					source: e,
					path: ctx.scope.resource.clone(),
					target: Some(self.0.clone()),
					context: ErrorContext::from_scope(&ctx.scope),
				})?;
				Ok(())
			}
			Err(e) => Err(Error::Io {
				source: e,
				path: ctx.scope.resource.clone(),
				target: Some(self.0.clone()),
				context: ErrorContext::from_scope(&ctx.scope),
			}),
		}
	}
}
