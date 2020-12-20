use crate::data::config::actions::io_action::{ConflictOption, Sep};
use std::path::PathBuf;

pub trait Update {
	fn update(self, if_exists: &ConflictOption, sep: &Sep) -> Option<PathBuf>;
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
	fn update(self, if_exists: &ConflictOption, sep: &Sep) -> Option<PathBuf> {
		match if_exists {
			ConflictOption::Skip => None,
			ConflictOption::Overwrite => Some(self.into()),
			ConflictOption::Rename => {
				let mut path = self.into();
				let extension = path.extension().unwrap_or_default().to_string_lossy().to_string();
				let stem = path.file_stem()?.to_string_lossy().to_string();
				let mut n = 1;
				while path.exists() {
					path.set_file_name(format!("{}{}({:?}).{}", stem, sep.as_str(), n, extension));
					n += 1;
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
		let new_path = original.update(&ConflictOption::Rename, &Default::default()).unwrap();
		assert_eq!(new_path, expected)
	}

	#[test]
	fn rename_with_overwrite_conflict() {
		let original = project().join("tests").join("files").join("test2.txt");
		let new_path = original.clone().update(&ConflictOption::Overwrite, &Default::default()).unwrap();
		assert_eq!(new_path, original)
	}

	#[test]
	fn rename_with_skip_conflict() {
		let original = project().join("tests").join("files").join("test2.txt");
		assert!(original.update(&ConflictOption::Skip, &Default::default()).is_none())
	}
}
