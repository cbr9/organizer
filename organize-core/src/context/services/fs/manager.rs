use crate::{
	context::{services::fs::locker::Locker, ExecutionContext},
	errors::{Error, ErrorContext},
	resource::Resource,
	templates::template::Template,
};
use anyhow::Result;
use path_clean::PathClean;
use serde::{Deserialize, Serialize};
use std::{
	path::{Path, PathBuf},
	sync::Arc,
}; // Assuming this is needed for dry_run and context

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Destination {
	pub folder: Template,
	pub filename: Option<Template>,
}

impl Destination {
	pub async fn get_final_path(&self, ctx: &ExecutionContext<'_>) -> Result<PathBuf, Error> {
		let folder = ctx.services.templater.render(&self.folder, ctx).await?;

		let mut folder = PathBuf::from(folder).clean();
		let filename = if let Some(filename) = &self.filename {
			ctx.services.templater.render(filename, ctx).await?
		} else {
			ctx.scope.resource.file_name().unwrap().to_string_lossy().to_string()
		};

		let filename = PathBuf::from(filename).clean();
		folder.push(filename);
		Ok(folder)
	}
}

#[derive(Debug, Clone, Default)]
pub struct FileSystemManager {
	pub locker: Locker,
}

impl FileSystemManager {
	pub async fn ensure_parent_dir_exists(&self, path: &Path) -> std::io::Result<()> {
		if let Some(parent) = path.parent() {
			if !tokio::fs::try_exists(parent).await.unwrap_or(false) {
				tokio::fs::create_dir_all(parent).await?;
			}
		}
		Ok(())
	}

	pub async fn r#move(
		&self,
		source: Arc<Resource>,
		destination: Arc<Resource>,
		ctx: &ExecutionContext<'_>, // Pass context for error reporting and dry_run
	) -> Result<(), Error> {
		// Attempt a direct rename first
		self.ensure_parent_dir_exists(&destination).await.map_err(|e| Error::Io {
			source: e,
			path: source.clone(),
			target: Some(destination.clone()),
			context: ErrorContext::from_scope(&ctx.scope),
		})?;

		match tokio::fs::rename(source.as_path(), destination.as_path()).await {
			Ok(_) => Ok(()),
			Err(e) if e.raw_os_error() == Some(libc::EXDEV) || e.kind() == std::io::ErrorKind::CrossesDevices => {
				// Handle "Cross-device link" error (EXDEV on Unix, specific error kind on Windows)
				// This means source and destination are on different file systems.
				tracing::warn!(
					"Attempting copy-then-delete for move operation due to cross-device link: {} to {}",
					source.display(),
					destination.display()
				);

				// Perform copy
				tokio::fs::copy(source.as_path(), destination.as_path())
					.await
					.map_err(|io_err| Error::Io {
						source: io_err,
						path: source.clone(),
						target: Some(destination.clone()),
						context: ErrorContext::from_scope(&ctx.scope),
					})?;

				// If copy is successful, delete the original
				tokio::fs::remove_file(source.as_path()).await.map_err(|io_err| Error::Io {
					source: io_err,
					path: source.clone(),
					target: None,
					context: ErrorContext::from_scope(&ctx.scope),
				})
			}
			Err(e) => {
				// Other I/O errors
				Err(Error::Io {
					source: e,
					path: source.clone(),
					target: Some(destination.clone()),
					context: ErrorContext::from_scope(&ctx.scope),
				})
			}
		}
	}

	pub async fn r#move_no_ctx(&self, source: Arc<Resource>, destination: Arc<Resource>) -> Result<()> {
		// Attempt a direct rename first
		self.ensure_parent_dir_exists(&destination).await?;

		match tokio::fs::rename(source.as_path(), destination.as_path()).await {
			Ok(_) => Ok(()),
			Err(e) if e.raw_os_error() == Some(libc::EXDEV) || e.kind() == std::io::ErrorKind::CrossesDevices => {
				// Handle "Cross-device link" error (EXDEV on Unix, specific error kind on Windows)
				// This means source and destination are on different file systems.
				tracing::warn!(
					"Attempting copy-then-delete for move operation due to cross-device link: {} to {}",
					source.display(),
					destination.display()
				);

				// Perform copy
				tokio::fs::copy(source.as_path(), destination.as_path()).await?;

				// If copy is successful, delete the original
				Ok(tokio::fs::remove_file(source.as_path()).await?)
			}
			Err(e) => {
				// Other I/O errors
				Err(e.into())
			}
		}
	}
}
