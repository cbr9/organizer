use crate::{
	config::context::ExecutionContext,
	errors::{Error, ErrorContext},
	resource::Resource,
};
use anyhow::Result;
use std::path::Path; // Assuming this is needed for dry_run and context

pub async fn ensure_parent_dir_exists(path: &Path) -> std::io::Result<()> {
	if let Some(parent) = path.parent() {
		if !tokio::fs::try_exists(parent).await.unwrap_or(false) {
			tokio::fs::create_dir_all(parent).await?;
		}
	}
	Ok(())
}

pub async fn move_file(source: &Resource, destination: &Resource) -> std::io::Result<()> {
	ensure_parent_dir_exists(destination).await?;
	match tokio::fs::rename(source, destination).await {
		Err(e) if e.raw_os_error() == Some(libc::EXDEV) || e.kind() == std::io::ErrorKind::CrossesDevices => {
			// Handle "Cross-device link" error (EXDEV on Unix, specific error kind on Windows)
			// This means source and destination are on different file systems.
			tracing::warn!(
				"Attempting copy-then-delete for move operation due to cross-device link: {} to {}",
				source.display(),
				destination.display()
			);

			tokio::fs::copy(source, destination).await?;
			tokio::fs::remove_file(source).await
		}
		Err(e) => Err(e),
		Ok(_) => Ok(()),
	}
}

pub async fn r#move(
	source: &Resource,
	destination: &Resource,
	ctx: &ExecutionContext<'_>, // Pass context for error reporting and dry_run
) -> Result<(), Error> {
	// Attempt a direct rename first
	ensure_parent_dir_exists(destination).await.map_err(|e| Error::Io {
		source: e,
		path: source.clone(),
		target: Some(destination.clone()),
		context: ErrorContext::from_scope(&ctx.scope),
	})?;

	match tokio::fs::rename(source, destination).await {
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
			tokio::fs::copy(source, destination).await.map_err(|io_err| Error::Io {
				source: io_err,
				path: source.clone(),
				target: Some(destination.clone()),
				context: ErrorContext::from_scope(&ctx.scope),
			})?;

			// If copy is successful, delete the original
			tokio::fs::remove_file(source).await.map_err(|io_err| Error::Io {
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
