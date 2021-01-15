use crate::data::config::actions::io_action::{ConflictOption, Sep};


use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use crate::simulation::Simulation;

pub trait Update {
	fn update(self, if_exists: &ConflictOption, sep: &Sep, simulation: Option<&Arc<Mutex<Simulation>>>) -> Option<PathBuf>;
}

impl<T: Into<PathBuf>> Update for T {
	///  When trying to rename a file to a path that already exists, calling update() on the
	///  target path will return a new valid path.
	///  # Args
	/// * `if_exists`: option to resolve the naming conflict
	/// * `sep`: if `if_exists` is set to rename, `sep` will go between the filename and the added counter
	/// * `is_watching`: whether this function is being run from a watcher or not
	/// # Return
	/// This function will return `Some(new_path)` if `if_exists` is not set to skip, otherwise it returns `None`
	fn update(self, if_exists: &ConflictOption, sep: &Sep, simulation: Option<&Arc<Mutex<Simulation>>>) -> Option<PathBuf> {
		use ConflictOption::*;
		match if_exists {
			Skip | Delete => None,
			Overwrite => Some(self.into()),
			Rename => {
				let mut path = self.into();
				let extension = path.extension().unwrap_or_default().to_string_lossy().to_string();
				let stem = path.file_stem()?.to_string_lossy().to_string();
				let mut n = 1;
				match simulation {
					None => {
						while path.exists() {
							path.set_file_name(format!("{}{}({:?}).{}", stem, sep.as_str(), n, extension));
							n += 1;
						}
					}
					Some(simulation) => {
                        let guard = simulation.lock().unwrap();
						let files = &guard.files;
						while files.contains(&path) {
							path.set_file_name(format!("{}{}({:?}).{}", stem, sep.as_str(), n, extension));
							n += 1;
						}
					}
				}
				Some(path)
			},
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
		let new_path = original.update(&ConflictOption::Rename, &Default::default(), None).unwrap();
		assert_eq!(new_path, expected)
	}

	#[test]
	fn rename_with_overwrite_conflict() {
		let original = project().join("tests").join("files").join("test2.txt");
		let new_path = original.clone().update(&ConflictOption::Overwrite, &Default::default(), None).unwrap();
		assert_eq!(new_path, original)
	}

	#[test]
	fn rename_with_skip_conflict() {
		let original = project().join("tests").join("files").join("test2.txt");
		assert!(original.update(&ConflictOption::Skip, &Default::default(), None).is_none())
	}
}
