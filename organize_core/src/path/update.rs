use crate::data::config::actions::io_action::ConflictOption;

use crate::simulation::Simulation;
use std::path::PathBuf;
use std::sync::MutexGuard;

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
	use crate::utils::tests::project;

	#[test]
	fn rename_with_rename_conflict() {
		let original = project().join("tests").join("files").join("test2.txt");
		let expected = original.with_file_name("test2 (1).txt");
		let new_path = original.resolve_naming_conflict(&ConflictOption::default(), None).unwrap();
		assert_eq!(new_path, expected)
	}

	#[test]
	fn rename_with_overwrite_conflict() {
		let original = project().join("tests").join("files").join("test2.txt");
		let new_path = original.clone().resolve_naming_conflict(&ConflictOption::Overwrite, None).unwrap();
		assert_eq!(new_path, original)
	}

	#[test]
	fn rename_with_skip_conflict() {
		let original = project().join("tests").join("files").join("test2.txt");
		assert!(original.resolve_naming_conflict(&ConflictOption::Skip, None).is_none())
	}
}
