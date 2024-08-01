use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Defines the options available to resolve a naming conflict,
/// i.e. how the application should proceed when a file exists
/// but it should move/rename/copy some file to that existing path
#[derive(Eq, PartialEq, Default, Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum ConflictOption {
	Overwrite,
	Skip,
	#[default]
	Rename,
}

impl ConflictOption {
	pub fn resolve_naming_conflict<T: AsRef<Path>>(&self, target_path: T) -> Option<PathBuf> {
		use ConflictOption::*;
		let mut path = target_path.as_ref().to_path_buf();
		match self {
			Skip => None,
			Overwrite => Some(path.to_path_buf()),
			Rename => {
				let counter_separator = " ";
				let extension = path.extension().unwrap_or_default().to_string_lossy().to_string();
				let stem = path.file_stem()?.to_string_lossy().to_string();
				let mut n = 1;
				while path.exists() {
					if extension.is_empty() {
						path.set_file_name(format!("{}{}({:?})", stem, counter_separator, n));
					} else {
						path.set_file_name(format!("{}{}({:?}).{}", stem, counter_separator, n, extension));
					}
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
	use pretty_assertions::assert_eq;
	use tempfile::{Builder, NamedTempFile};

	#[test]
	fn skip() {
		let path = PathBuf::from("/home/user/skipped_file.txt");
		let new = ConflictOption::Skip.resolve_naming_conflict(path);
		assert_eq!(new, None)
	}

	#[test]
	fn overwrite() {
		let path = PathBuf::from("/home/user/skipped_file.txt");
		let new = ConflictOption::Overwrite.resolve_naming_conflict(&path);
		assert_eq!(new, Some(path))
	}

	#[test]
	fn rename_no_extension() {
		let file = NamedTempFile::new().unwrap();
		let path = file.path().to_path_buf();
		let file_name = path.file_stem().unwrap().to_string_lossy();
		let mut expected = path.clone();
		expected.set_file_name(format!("{} (1)", file_name));
		let new = ConflictOption::Rename.resolve_naming_conflict(&path);
		assert_eq!(new, Some(expected))
	}

	#[test]
	fn rename_extension() {
		let file = Builder::new().suffix(".txt").tempfile().unwrap();
		let path = file.path().to_path_buf();
		let file_name = path.file_stem().unwrap().to_string_lossy();
		dbg!(file_name.clone());
		let mut expected = path.clone();
		expected.set_file_name(format!("{} (1).txt", file_name));
		let new = ConflictOption::Rename.resolve_naming_conflict(&path);
		assert_eq!(new, Some(expected))
	}
}
