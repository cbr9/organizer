#[cfg(target_os = "windows")]
use std::os::windows::prelude::*;
#[cfg(target_os = "windows")]
use winapi::um::winnt::FILE_ATTRIBUTE_HIDDEN;

use std::path::Path;

pub trait IsHidden {
	fn is_hidden(&self) -> bool;
}

impl IsHidden for Path {
	#[cfg(any(target_os = "linux", target_os = "macos"))]
	fn is_hidden(&self) -> bool {
		match self.file_name() {
			None => false,
			Some(filename) => filename.to_string_lossy().starts_with('.'),
		}
	}

	#[cfg(target_os = "windows")]
	fn is_hidden(&self) -> bool {
		let metadata = std::fs::metadata(self).unwrap();
		metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN > 0
	}
}

#[cfg(test)]
mod tests {
	use crate::path::IsHidden;
	#[cfg(target_os = "windows")]
	use std::os::windows::prelude::*;
	use std::{fs, path::Path};
	#[cfg(target_os = "windows")]
	use winapi::um::winnt::FILE_ATTRIBUTE_HIDDEN;

	#[test]
	#[cfg(any(target_os = "linux", target_os = "macos"))]
	fn check_hidden() {
		let path = Path::new(".testfile");
		assert!(path.is_hidden())
	}

	#[test]
	#[cfg(target_os = "windows")]
	fn check_hidden() {
		let path = Path::new(".testfile");
		fs::OpenOptions::new()
			.create(true)
			.write(true)
			.open(path)
			.custom_flags(FILE_ATTRIBUTE_HIDDEN);
		assert!(path.is_hidden())
	}
}
