use std::path::Path;

use anyhow::Result;

pub trait IsHidden {
	type Err;
	fn is_hidden(&self) -> Result<bool, Self::Err>;
}

#[cfg(target_family = "unix")]
impl IsHidden for Path {
	type Err = std::convert::Infallible;

	fn is_hidden(&self) -> Result<bool, Self::Err> {
		match self.file_name() {
			None => Ok(false),
			Some(filename) => Ok(filename.to_string_lossy().starts_with('.')),
		}
	}
}

#[cfg(target_family = "windows")]
impl IsHidden for Path {
	type Err = std::io::Error;

	fn is_hidden(&self) -> Result<bool, Self::Err> {
		use std::{fs, os::windows::prelude::*};
		let metadata = fs::metadata(self)?;
		let attributes = metadata.file_attributes();
		Ok((attributes & 0x2) > 0)
	}
}

#[cfg(test)]
mod tests {
	#[cfg(target_family = "unix")]
	#[test]
	fn check_hidden() {
		use super::*;
		let path = Path::new("/home/user/.testfile");
		assert!(path.is_hidden().unwrap())
	}

	#[cfg(target_family = "windows")]
	#[test]
	fn not_hidden() {
		use tempfile::NamedTempFile;

		use super::*;
		let file = NamedTempFile::new().unwrap();
		let path = file.path();
		assert!(!path.is_hidden().unwrap());
	}

	#[test]
	#[cfg(target_family = "windows")]
	fn check_hidden() {
		use tempfile::NamedTempFile;

		use crate::path::is_hidden::IsHidden;

		let file = NamedTempFile::new().unwrap();
		let path = file.path();
		// Use the `attrib` command on Windows to set the hidden attribute.
		let status = std::process::Command::new("attrib")
			.arg("+h")
			.arg(path.as_os_str())
			.status()
			.expect("failed to execute attrib command");
		assert!(status.success(), "attrib command failed");
		assert!(path.is_hidden().unwrap());
	}
}
