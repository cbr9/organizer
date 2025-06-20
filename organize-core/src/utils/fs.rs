// organize-core/src/utils/fs_ops.rs (new file)

use crate::{
	config::context::ExecutionContext,
	errors::{Error, ErrorContext},
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

pub async fn move_safely(
	source: &Path,
	destination: &Path,
	ctx: &ExecutionContext<'_>, // Pass context for error reporting and dry_run
) -> Result<(), Error> {
	// Attempt a direct rename first
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

			ensure_parent_dir_exists(destination).await.map_err(|e| Error::Io {
				source: e,
				path: source.to_path_buf(),
				target: Some(destination.to_path_buf()),
				context: ErrorContext::from_scope(&ctx.scope),
			})?;

			// Perform copy
			tokio::fs::copy(source, destination).await.map_err(|io_err| Error::Io {
				source: io_err,
				path: source.to_path_buf(),
				target: Some(destination.to_path_buf()),
				context: ErrorContext::from_scope(&ctx.scope),
			})?;

			// If copy is successful, delete the original
			tokio::fs::remove_file(source).await.map_err(|io_err| Error::Io {
				source: io_err,
				path: source.to_path_buf(),
				target: None,
				context: ErrorContext::from_scope(&ctx.scope),
			})
		}
		Err(e) => {
			// Other I/O errors
			Err(Error::Io {
				source: e,
				path: source.to_path_buf(),
				target: Some(destination.to_path_buf()),
				context: ErrorContext::from_scope(&ctx.scope),
			})
		}
	}
}

/// Checks whether a file exists asynchronously.
/// If in dry_run mode, it checks the simulated paths in the blackboard.
/// Otherwise, it performs an actual file system check.
pub async fn try_exists(path: &Path, ctx: &ExecutionContext<'_>) -> Result<bool, Error> {
	if ctx.settings.dry_run {
		// In dry_run mode, check against the set of simulated paths
		Ok(ctx.services.blackboard.known_paths.contains(path))
	} else {
		// In actual run mode, perform a real file system check
		tokio::fs::try_exists(path).await.map_err(|e| Error::Io {
			source: e,
			path: path.to_path_buf(),
			target: None,
			context: ErrorContext::from_scope(&ctx.scope),
		})
	}
}
