use anyhow::Context;

use crate::config::actions::io_action::ConflictOption;

use std::path::PathBuf;

pub trait ResolveConflict {
	fn resolve_naming_conflict(self, on_conflict: &ConflictOption) -> Option<PathBuf>;
}

impl<T: Into<PathBuf>> ResolveConflict for T {
	fn resolve_naming_conflict(self, on_conflict: &ConflictOption) -> Option<PathBuf> {
		use ConflictOption::*;
		let mut path = self.into();
		match on_conflict {
			Delete => {
				if let Err(e) = std::fs::remove_file(&path).with_context(|| format!("could not delete {}", path.display())) {
					log::error!("{:?}", e);
				}
				None
			}
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
		let new = path.resolve_naming_conflict(&ConflictOption::Skip);
		assert_eq!(new, None)
	}

	#[test]
	fn delete() {
		let file = NamedTempFile::new().unwrap();
		let tmp_path = file.into_temp_path();
		let new = tmp_path.resolve_naming_conflict(&ConflictOption::Delete);
		assert_eq!(new, None);
		assert!(!tmp_path.exists());
	}

	#[test]
	fn overwrite() {
		let path = PathBuf::from("/home/user/skipped_file.txt");
		let new = path.clone().resolve_naming_conflict(&ConflictOption::Overwrite);
		assert_eq!(new, Some(path))
	}

	#[test]
	fn rename_no_extension() {
		let file = NamedTempFile::new().unwrap();
		let path = file.path().to_path_buf();
		let file_name = path.file_stem().unwrap().to_string_lossy();
		let mut expected = path.clone();
		expected.set_file_name(format!("{} (1)", file_name));
		let new = path.clone().resolve_naming_conflict(&ConflictOption::Rename);
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
		let new = path.clone().resolve_naming_conflict(&ConflictOption::Rename);
		assert_eq!(new, Some(expected))
	}
}
