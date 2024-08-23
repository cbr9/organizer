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

	#[tracing::instrument(err)]
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
		use super::*;
		let path = Path::new("/home/user/testfile");
		assert!(!path.is_hidden().unwrap())
	}
}
