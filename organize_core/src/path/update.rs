use crate::data::config::actions::io_action::ConflictOption;

use crate::simulation::Simulation;
use std::{path::PathBuf, sync::MutexGuard};

pub trait ResolveConflict {
	fn resolve_naming_conflict(self, if_exists: &ConflictOption, simulation: Option<MutexGuard<Simulation>>) -> Option<PathBuf>;
}

impl<T: Into<PathBuf>> ResolveConflict for T {
	fn resolve_naming_conflict(self, if_exists: &ConflictOption, simulation: Option<MutexGuard<Simulation>>) -> Option<PathBuf> {
		use ConflictOption::*;
		match if_exists {
			Skip | Delete => None,
			Overwrite => Some(self.into()),
			Rename { counter_separator } => {
				let mut path = self.into();
				let extension = path.extension().unwrap_or_default().to_string_lossy().to_string();
				let stem = path.file_stem()?.to_string_lossy().to_string();
				let mut n = 1;
				match simulation {
					None => {
						while path.exists() {
							path.set_file_name(format!("{}{}({:?}).{}", stem, counter_separator.as_str(), n, extension));
							n += 1;
						}
					}
					Some(simulation) => {
						while simulation.files.contains(&path) {
							path.set_file_name(format!("{}{}({:?}).{}", stem, counter_separator.as_str(), n, extension));
							n += 1;
						}
					}
				}
				Some(path)
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::utils::tests::{AndWait, TEST_FILES_DIRECTORY};
	use anyhow::Result;
	use std::fs::File;

	#[test]
	fn rename_non_existent() {
		let path = TEST_FILES_DIRECTORY.join("rename_non_existent.txt");
		let option = ConflictOption::Rename {
			counter_separator: " ".to_string(),
		};
		assert_eq!(path, path.clone().resolve_naming_conflict(&option, None).unwrap());
	}
	#[test]
	fn rename_existent() -> Result<()> {
		let path = TEST_FILES_DIRECTORY.join("rename_existent.txt");
		File::create_and_wait(&path)?;
		let option = ConflictOption::Rename {
			counter_separator: " ".to_string(),
		};
		let new_path = path.clone().resolve_naming_conflict(&option, None).unwrap();
		let is_ok = new_path == TEST_FILES_DIRECTORY.join("rename_existent (1).txt");
		File::remove_and_wait(path)?;
		assert!(is_ok);
		Ok(())
	}
	#[test]
	fn rename_existent_simulated() -> Result<()> {
		let simulation = Simulation::new()?;
		let path = TEST_FILES_DIRECTORY.join("test.txt");
		{
			let mut guard = simulation.lock().unwrap();
			guard.insert_file(&path);
		}
		let option = ConflictOption::Rename {
			counter_separator: " ".to_string(),
		};
		let new_path = path.resolve_naming_conflict(&option, Some(simulation.lock().unwrap())).unwrap();
		assert_eq!(new_path, TEST_FILES_DIRECTORY.join("test (1).txt"));
		Ok(())
	}
	#[test]
	fn rename_non_existent_simulated() -> Result<()> {
		let simulation = Simulation::new()?;
		let path = TEST_FILES_DIRECTORY.join("test.txt");
		let option = ConflictOption::Rename {
			counter_separator: " ".to_string(),
		};
		let new_path = path.resolve_naming_conflict(&option, Some(simulation.lock().unwrap())).unwrap();
		assert_eq!(new_path, TEST_FILES_DIRECTORY.join("test.txt"));
		Ok(())
	}
	#[test]
	fn overwrite() {
		let path = TEST_FILES_DIRECTORY.join("test.txt");
		let option = ConflictOption::Overwrite;
		assert_eq!(path, path.clone().resolve_naming_conflict(&option, None).unwrap());
	}

	#[test]
	fn skip() {
		let path = TEST_FILES_DIRECTORY.join("test.txt");
		assert!(path.resolve_naming_conflict(&ConflictOption::Skip, None).is_none())
	}

	#[test]
	fn delete() {
		let path = TEST_FILES_DIRECTORY.join("test.txt");
		assert!(path.resolve_naming_conflict(&ConflictOption::Delete, None).is_none())
	}
}
