use std::{convert::Infallible, path::Path};

use anyhow::{Error, Result};
use tracing::error_span;

pub trait IsHidden {
	type Err;
	fn is_hidden(&self) -> Result<bool, Self::Err>;
}

#[cfg(target_family = "unix")]
impl IsHidden for Path {
	fn is_hidden(&self) -> Result<bool, Self::Err> {
		match self.file_name() {
			None => Ok(false),
			Some(filename) => Ok(filename.to_string_lossy().starts_with('.')),
		}
	}

	type Err = Infallible;
}

#[cfg(target_family = "windows")]
impl IsHidden for Path {
	fn is_hidden(&self) -> bool {
		use std::{fs, os::windows::prelude::*};
		match fs::metadata(self) {
			Ok(metadata) => {
				let attributes = metadata.file_attributes();
				if (attributes & 0x2) > 0 {
					true
				} else {
					false
				}
			}
			Err(e) => {
				error!("{}", e);
				false
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[cfg(target_family = "unix")]
	#[test]
	fn check_hidden() {
		let path = Path::new("/home/user/.testfile");
		assert!(path.is_hidden().unwrap())
	}

	#[cfg(target_family = "unix")]
	#[test]
	fn not_hidden() {
		let path = Path::new("/home/user/testfile");
		assert!(!path.is_hidden().unwrap())
	}
}
