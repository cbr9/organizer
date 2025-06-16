use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Defines the options available to resolve a naming conflict,
/// i.e. how the application should proceed when a file exists
/// but it should move/rename/copy some file to that existing path
#[derive(Eq, PartialEq, Default, Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum ConflictOption {
	Overwrite,
	#[default]
	Skip,
	Rename,
}

pub fn enabled() -> bool {
	true
}

#[tracing::instrument(ret, level = "debug")]
pub fn resolve_naming_conflict<T: AsRef<Path> + std::fmt::Debug>(strategy: &ConflictOption, target_path: T) -> Option<PathBuf> {
	use ConflictOption::*;
	let mut path = target_path.as_ref().to_path_buf();
	if !path.exists() {
		return Some(path);
	}
	match strategy {
		Skip => None,
		Overwrite => Some(path),
		Rename => {
			let counter_separator = " ";
			let extension = path.extension().unwrap_or_default().to_string_lossy().to_string();
			let stem = path.file_stem()?.to_string_lossy().to_string();
			let mut n = 1;
			while path.exists() {
				if extension.is_empty() {
					path.set_file_name(format!("{stem}{counter_separator}({n:?})"));
				} else {
					path.set_file_name(format!("{stem}{counter_separator}({n:?}).{extension}"));
				}
				n += 1;
			}
			Some(path)
		}
	}
}

#[cfg(test)]
mod tests {

	use super::*;
	use pretty_assertions::assert_eq;
	use tempfile::{Builder, NamedTempFile};

	#[test]
	fn skip_exists() {
		let file = NamedTempFile::new().unwrap();
		let strategy = ConflictOption::Skip;
		let new = resolve_naming_conflict(&strategy, file.path());
		assert_eq!(new, None)
	}
	#[test]
	fn skip_not_exists() {
		let path = PathBuf::from("/home/user/skipped_file.txt");
		let new = resolve_naming_conflict(&ConflictOption::Skip, &path);
		assert_eq!(new, Some(path))
	}

	#[test]
	fn overwrite_exists() {
		let file = NamedTempFile::new().unwrap();
		let path = file.path();
		let new = resolve_naming_conflict(&ConflictOption::Overwrite, &path);
		assert_eq!(new, Some(path.to_path_buf()))
	}
	#[test]
	fn overwrite_not_exists() {
		let path = PathBuf::from("/home/user/skipped_file.txt");
		let new = resolve_naming_conflict(&ConflictOption::Overwrite, &path);
		assert_eq!(new, Some(path))
	}

	#[test]
	fn rename_no_extension() {
		let file = NamedTempFile::new().unwrap();
		let path = file.path().to_path_buf();
		let file_name = path.file_stem().unwrap().to_string_lossy();
		let mut expected = path.clone();
		expected.set_file_name(format!("{} (1)", file_name));
		let new = resolve_naming_conflict(&ConflictOption::Rename, &path);
		assert_eq!(new, Some(expected))
	}

	#[test]
	fn rename_extension() {
		let file = Builder::new().suffix(".txt").tempfile().unwrap();
		let path = file.path().to_path_buf();
		let file_name = path.file_stem().unwrap().to_string_lossy();
		let mut expected = path.clone();
		expected.set_file_name(format!("{} (1).txt", file_name));
		let new = resolve_naming_conflict(&ConflictOption::Rename, &path);
		assert_eq!(new, Some(expected))
	}
}
