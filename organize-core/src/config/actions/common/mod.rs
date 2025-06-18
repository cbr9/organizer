use std::{
	fs::{self, File},
	path::{Path, PathBuf},
};

use path_clean::PathClean;
use serde::{Deserialize, Serialize};
use strum::Display;
/// Represents an exclusive, temporary reservation of a filesystem path.
/// The OS-level lock is held as long as this struct exists, and it is released when dropped.
/// This prevents race conditions without creating orphaned placeholder files.
#[derive(Debug)]
pub struct GuardedPath {
	path: PathBuf,
	_lock: Option<Lock>, // The locked file handle. Its existence in the struct keeps the lock alive.
}

impl std::ops::Deref for GuardedPath {
	type Target = PathBuf;

	fn deref(&self) -> &Self::Target {
		&self.path
	}
}

impl GuardedPath {
	pub fn to_path_buf(self) -> PathBuf {
		self.path
	}
}

impl AsRef<Path> for GuardedPath {
	fn as_ref(&self) -> &Path {
		self.path.as_path()
	}
}

#[derive(Debug)]
struct Lock {
	_file: File,
	path: PathBuf,
}

impl std::ops::Deref for Lock {
	type Target = File;

	fn deref(&self) -> &Self::Target {
		&self._file
	}
}

impl Lock {
	fn new(path: impl AsRef<Path>) -> anyhow::Result<Self> {
		let path = path.as_ref().with_added_extension("lock");
		Ok(Self {
			_file: fs::OpenOptions::new().write(true).truncate(true).create(true).open(&path)?,
			path,
		})
	}
}

impl Drop for Lock {
	fn drop(&mut self) {
		let _ = fs::remove_file(&self.path);
	}
}

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

fn get_renamed_path(mut path: PathBuf) -> PathBuf {
	let counter_separator = " ";
	// Store original stem and extension to prevent them from being changed in the loop
	let original_stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
	let original_extension = path.extension().unwrap_or_default().to_string_lossy().to_string();

	let mut n = 1;
	while path.exists() {
		let new_name = if original_extension.is_empty() {
			format!("{original_stem}{counter_separator}({n})")
		} else {
			format!("{original_stem}{counter_separator}({n}).{original_extension}")
		};
		path.set_file_name(new_name);
		n += 1;
	}
	path
}

impl ConflictResolution {
	/// Simulates the resolution of a naming conflict without any filesystem side effects.
	/// This is suitable for a dry run.
	#[tracing::instrument(ret, level = "debug")]
	pub fn resolve<T: AsRef<Path> + std::fmt::Debug>(&self, target_path: T) -> Option<GuardedPath> {
		let path = target_path.as_ref().to_path_buf();
		if !path.exists() {
			return Some(GuardedPath { path, _lock: None });
		}

		let path = match self {
			ConflictResolution::Skip => return None,
			ConflictResolution::Overwrite => path,
			ConflictResolution::Rename => get_renamed_path(path),
		};

		Some(GuardedPath { path, _lock: None })
	}

	#[tracing::instrument(ret, level = "debug")]
	pub fn resolve_atomic<T: AsRef<Path> + std::fmt::Debug>(&self, target_path: T) -> Option<GuardedPath> {
		let mut path_to_try = target_path.as_ref().to_path_buf();

		loop {
			// 1. DETERMINE THE NEXT PATH TO ATTEMPT
			// On the first loop, this is the original target_path.
			// On subsequent loops for Rename, it will be an incremented path.

			// 2. ACQUIRE A LOCK FOR THAT SPECIFIC PATH
			// This is the most critical step. We lock a file corresponding to the path we *intend* to check.

			if let Some(parent) = path_to_try.parent() {
				if fs::create_dir_all(parent).is_err() {
					return None;
				}
			}

			let lock = Lock::new(&path_to_try).ok()?;

			if lock.lock().is_err() {
				// Could not get the lock. This means another thread is currently processing this exact path.
				// If our strategy is Rename, we should try the next name. Otherwise, we can't proceed.
				if let ConflictResolution::Rename = self {
					path_to_try = get_renamed_path(path_to_try);
					continue; // Try the new name in the next loop iteration.
				} else {
					return None; // For Skip or Overwrite, contention means we stop.
				}
			}

			// --- CRITICAL SECTION ---
			// We now have an exclusive lock on the .lock file for `path_to_try`.
			// No other thread can be evaluating this same path.

			// 3. CHECK THE ACTUAL PATH'S EXISTENCE
			if path_to_try.exists() {
				match self {
					ConflictResolution::Skip => return None, // Release lock by returning
					ConflictResolution::Overwrite => {
						// The path exists and we will overwrite it. The reservation is valid.
						return Some(GuardedPath {
							path: path_to_try.clean(),
							_lock: Some(lock),
						});
					}
					ConflictResolution::Rename => {
						// We have a lock, but the file exists. This can happen in a race if
						// another process created the file after we locked.
						// We release the lock (by letting lock_file go out of scope) and try again.
						path_to_try = get_renamed_path(path_to_try);
						continue;
					}
				}
			} else {
				// The path is free. The reservation is valid.
				return Some(GuardedPath {
					path: path_to_try.clean(),
					_lock: Some(lock),
				});
			}
		}
	}
}

#[cfg(test)]
mod tests {

	use super::*;
	use pretty_assertions::assert_eq;
	use std::{collections::HashSet, sync::Arc, thread};
	use tempfile::{Builder, NamedTempFile};

	// -- Tests for resolve() (dry run simulation) --

	#[test]
	fn resolve_skip_exists() {
		let file = NamedTempFile::new().unwrap();
		let strategy = ConflictResolution::Skip;
		let reservation = strategy.resolve(file.path());
		assert!(reservation.is_none());
	}
	#[test]
	fn resolve_skip_not_exists() {
		let path = PathBuf::from("a-file-that-does-not-exist.tmp");
		let strategy = ConflictResolution::Skip;
		let reservation = strategy.resolve(&path);
		assert_eq!(reservation.map(|r| r.path.clone()), Some(path));
	}

	#[test]
	fn resolve_overwrite_exists() {
		let file = NamedTempFile::new().unwrap();
		let path = file.path();
		let strategy = ConflictResolution::Overwrite;
		let reservation = strategy.resolve(&path);
		assert_eq!(reservation.map(|r| r.path.clone()), Some(path.to_path_buf()));
	}

	#[test]
	fn resolve_rename_extension() {
		let dir = tempfile::tempdir().unwrap();
		let file = Builder::new().suffix(".txt").tempfile_in(dir.path()).unwrap();
		let path = file.path().to_path_buf();
		let file_name = path.file_stem().unwrap().to_string_lossy();
		let mut expected = path.clone();
		expected.set_file_name(format!("{} (1).txt", file_name));
		let strategy = ConflictResolution::Rename;
		let reservation = strategy.resolve(&path);
		assert_eq!(reservation.map(|r| r.path.clone()), Some(expected));
	}

	// -- Tests for resolve_atomic() (real run) --

	#[test]
	fn atomic_skip_exists() {
		let file = NamedTempFile::new().unwrap();
		let strategy = ConflictResolution::Skip;
		let reservation = strategy.resolve_atomic(file.path());
		assert!(reservation.is_none());
	}

	#[test]
	fn atomic_overwrite_exists() {
		let file = NamedTempFile::new().unwrap();
		let path = file.path().to_path_buf();
		let strategy = ConflictResolution::Overwrite;
		let reservation = strategy.resolve_atomic(&path).unwrap();
		assert_eq!(reservation.path, path);
		assert!(reservation._lock.is_some());
	}

	#[test]
	fn atomic_rename_single_thread() {
		let dir = tempfile::tempdir().unwrap();
		let file = Builder::new().suffix(".txt").tempfile_in(dir.path()).unwrap();
		let path = file.path().to_path_buf();
		let file_name = path.file_stem().unwrap().to_string_lossy();
		let mut expected = path.clone();
		expected.set_file_name(format!("{} (1).txt", file_name));
		let strategy = ConflictResolution::Rename;

		let reservation = strategy.resolve_atomic(&path).unwrap();
		assert_eq!(reservation.path, expected);
		assert!(reservation._lock.is_some());
	}

	#[test]
	fn atomic_rename_concurrently_produces_unique_names() {
		let dir = tempfile::tempdir().unwrap();
		let conflict_path = Arc::new(dir.path().join("concurrent_test.txt"));
		fs::write(&*conflict_path, "existing file").unwrap();

		let mut handles = Vec::new();
		let num_threads = 10;

		for _ in 0..num_threads {
			let path_clone = Arc::clone(&conflict_path);
			let handle = thread::spawn(move || {
				// THE FIX IS HERE:
				// 1. The thread calls resolve_atomic to get the reservation.
				let reservation_option = ConflictResolution::Rename.resolve_atomic(&*path_clone);

				// 2. It then extracts the `path` from the reservation. Because `map` takes
				//    ownership, the `PathReservation` object is dropped immediately after
				//    the path is extracted, releasing the OS lock from within the thread.
				// 3. The thread only returns the PathBuf, NOT the reservation guard.
				if let Some(ref reservation) = reservation_option {
					// THE FIX, AS YOU SUGGESTED:
					// We must simulate the action that would follow, which is creating a file
					// at the resolved path. This ensures the next thread will see this
					// file and generate a different name.
					// NOTE: In the real app, this is done by the Move/Copy action. Here, we
					//       are overwriting the 0-byte placeholder with some content.
					fs::write(&reservation.path, "simulation of a moved file").unwrap();
				}
				reservation_option.map(|reservation| reservation.path.clone())
			});
			handles.push(handle);
		}

		// The 'results' vector will now correctly store PathBufs, not PathReservations.
		let mut results = Vec::new();
		for handle in handles {
			// Each thread should succeed and return `Some(PathBuf)`.
			// The result of join is Result<Option<PathBuf>, Error>
			let path_option = handle.join().unwrap();
			results.push(path_option.unwrap());
		}

		// Verify that all 10 threads got a result and that every single path is unique.
		let unique_paths: HashSet<_> = results.iter().cloned().collect();
		assert_eq!(results.len(), num_threads, "All threads should successfully get a path.");
		assert_eq!(unique_paths.len(), num_threads, "All resolved paths must be unique.");
	}
}
