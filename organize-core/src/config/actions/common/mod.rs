use anyhow::anyhow;

use serde::{Deserialize, Serialize};
use strum::Display;

use crate::{
	config::context::ExecutionContext,
	resource::Resource,
};

/// Represents an exclusive, temporary reservation of a filesystem path.
/// The OS-level lock is held as long as this struct exists, and it is released when dropped.
/// This prevents race conditions without creating orphaned placeholder files.
/// Defines the options available to resolve a naming conflict,
/// i.e. how the application should proceed when a file exists
/// but it should move/rename/copy some file to that existing path
#[derive(Eq, Display, PartialEq, Default, Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
#[strum(serialize_all = "snake_case")]
pub enum ConflictResolution {
	Overwrite,
	#[default]
	Skip,
	Rename,
}

pub fn enabled() -> bool {
	true
}

impl ConflictResolution {
	/// Asynchronously resolves a path conflict based on the policy.
	///
	/// This will check if the given path exists on the filesystem and,
	/// depending on the variant, will either return a new, non-conflicting
	/// path, `None` (to signal a skip), or the original path (for overwrite).
	pub async fn resolve(&self, mut path: Resource, ctx: &ExecutionContext<'_>) -> anyhow::Result<Option<Resource>> {
		// Use the non-blocking `try_exists` and .await the result.
		if !path.try_exists(ctx).await? {
			// If the path doesn't exist, there is no conflict.
			return Ok(Some(path.clone()));
		}

		match self {
			ConflictResolution::Overwrite => Ok(Some(path.clone())),
			ConflictResolution::Skip => Ok(None),
			ConflictResolution::Rename => {
				let mut n = 1;

				// The loop condition now uses the async `try_exists`.
				while path.try_exists(ctx).await? {
					// This logic is made more robust to handle files without stems or extensions.
					let stem = path
						.file_stem()
						.and_then(|s| s.to_str())
						.ok_or_else(|| anyhow!("Cannot get file stem from path: {}", path.display()))?;

					let new_name = match path.extension().and_then(|s| s.to_str()) {
						Some(extension) => format!("{stem} ({n}).{extension}"),
						None => format!("{stem} ({n})"),
					};

					path = path.with_file_name(new_name).into();
					n += 1;
				}
				Ok(Some(path))
			}
		}
	}
}

// #[cfg(test)]
// mod tests {

// 	use super::*;
// 	use pretty_assertions::assert_eq;
// 	use std::{collections::HashSet, sync::Arc, thread};
// 	use tempfile::{Builder, NamedTempFile};

// 	// -- Tests for resolve() (dry run simulation) --

// 	#[test]
// 	fn resolve_skip_exists() {
// 		let file = NamedTempFile::new().unwrap();
// 		let strategy = ConflictResolution::Skip;
// 		let reservation = strategy.resolve(file.path());
// 		assert!(reservation.is_none());
// 	}
// 	#[test]
// 	fn resolve_skip_not_exists() {
// 		let path = PathBuf::from("a-file-that-does-not-exist.tmp");
// 		let strategy = ConflictResolution::Skip;
// 		let reservation = strategy.resolve(&path);
// 		assert_eq!(reservation.map(|r| r.path.clone()), Some(path));
// 	}

// 	#[test]
// 	fn resolve_overwrite_exists() {
// 		let file = NamedTempFile::new().unwrap();
// 		let path = file.path();
// 		let strategy = ConflictResolution::Overwrite;
// 		let reservation = strategy.resolve(&path);
// 		assert_eq!(reservation.map(|r| r.path.clone()), Some(path.to_path_buf()));
// 	}

// 	#[test]
// 	fn resolve_rename_extension() {
// 		let dir = tempfile::tempdir().unwrap();
// 		let file = Builder::new().suffix(".txt").tempfile_in(dir.path()).unwrap();
// 		let path = file.path().to_path_buf();
// 		let file_name = path.file_stem().unwrap().to_string_lossy();
// 		let mut expected = path.clone();
// 		expected.set_file_name(format!("{} (1).txt", file_name));
// 		let strategy = ConflictResolution::Rename;
// 		let reservation = strategy.resolve(&path);
// 		assert_eq!(reservation.map(|r| r.path.clone()), Some(expected));
// 	}

// 	// -- Tests for resolve_atomic() (real run) --

// 	#[test]
// 	fn atomic_skip_exists() {
// 		let file = NamedTempFile::new().unwrap();
// 		let strategy = ConflictResolution::Skip;
// 		let reservation = strategy.resolve_atomic(file.path());
// 		assert!(reservation.is_none());
// 	}

// 	#[test]
// 	fn atomic_overwrite_exists() {
// 		let file = NamedTempFile::new().unwrap();
// 		let path = file.path().to_path_buf();
// 		let strategy = ConflictResolution::Overwrite;
// 		let reservation = strategy.resolve_atomic(&path).unwrap();
// 		assert_eq!(reservation.path, path);
// 		assert!(reservation._lock.is_some());
// 	}

// 	#[test]
// 	fn atomic_rename_single_thread() {
// 		let dir = tempfile::tempdir().unwrap();
// 		let file = Builder::new().suffix(".txt").tempfile_in(dir.path()).unwrap();
// 		let path = file.path().to_path_buf();
// 		let file_name = path.file_stem().unwrap().to_string_lossy();
// 		let mut expected = path.clone();
// 		expected.set_file_name(format!("{} (1).txt", file_name));
// 		let strategy = ConflictResolution::Rename;

// 		let reservation = strategy.resolve_atomic(&path).unwrap();
// 		assert_eq!(reservation.path, expected);
// 		assert!(reservation._lock.is_some());
// 	}

// 	#[test]
// 	fn atomic_rename_concurrently_produces_unique_names() {
// 		let dir = tempfile::tempdir().unwrap();
// 		let conflict_path = Arc::new(dir.path().join("concurrent_test.txt"));
// 		fs::write(&*conflict_path, "existing file").unwrap();

// 		let mut handles = Vec::new();
// 		let num_threads = 10;

// 		for _ in 0..num_threads {
// 			let path_clone = Arc::clone(&conflict_path);
// 			let handle = thread::spawn(move || {
// 				// THE FIX IS HERE:
// 				// 1. The thread calls resolve_atomic to get the reservation.
// 				let reservation_option = ConflictResolution::Rename.resolve_atomic(&*path_clone);

// 				// 2. It then extracts the `path` from the reservation. Because `map` takes
// 				//    ownership, the `PathReservation` object is dropped immediately after
// 				//    the path is extracted, releasing the OS lock from within the thread.
// 				// 3. The thread only returns the PathBuf, NOT the reservation guard.
// 				if let Some(ref reservation) = reservation_option {
// 					// THE FIX, AS YOU SUGGESTED:
// 					// We must simulate the action that would follow, which is creating a file
// 					// at the resolved path. This ensures the next thread will see this
// 					// file and generate a different name.
// 					// NOTE: In the real app, this is done by the Move/Copy action. Here, we
// 					//       are overwriting the 0-byte placeholder with some content.
// 					fs::write(&reservation.path, "simulation of a moved file").unwrap();
// 				}
// 				reservation_option.map(|reservation| reservation.path.clone())
// 			});
// 			handles.push(handle);
// 		}

// 		// The 'results' vector will now correctly store PathBufs, not PathReservations.
// 		let mut results = Vec::new();
// 		for handle in handles {
// 			// Each thread should succeed and return `Some(PathBuf)`.
// 			// The result of join is Result<Option<PathBuf>, Error>
// 			let path_option = handle.join().unwrap();
// 			results.push(path_option.unwrap());
// 		}

// 		// Verify that all 10 threads got a result and that every single path is unique.
// 		let unique_paths: HashSet<_> = results.iter().cloned().collect();
// 		assert_eq!(results.len(), num_threads, "All threads should successfully get a path.");
// 		assert_eq!(unique_paths.len(), num_threads, "All resolved paths must be unique.");
// 	}
// }
