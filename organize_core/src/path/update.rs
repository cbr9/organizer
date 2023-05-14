use crate::config::actions::io_action::ConflictOption;

use std::path::PathBuf;

pub trait ResolveConflict {
	fn resolve_naming_conflict(self, if_exists: &ConflictOption) -> Option<PathBuf>;
}

impl<T: Into<PathBuf>> ResolveConflict for T {
	fn resolve_naming_conflict(self, if_exists: &ConflictOption) -> Option<PathBuf> {
		use ConflictOption::*;
		match if_exists {
			Skip | Delete => None,
			Overwrite => Some(self.into()),
			Rename { counter_separator } => {
				let mut path = self.into();
				let extension = path.extension().unwrap_or_default().to_string_lossy().to_string();
				let stem = path.file_stem()?.to_string_lossy().to_string();
				let mut n = 1;
				while path.exists() {
					path.set_file_name(format!("{}{}({:?}).{}", stem, counter_separator.as_str(), n, extension));
					n += 1;
				}
				Some(path)
			}
		}
	}
}
