use std::path::Path;

pub trait IsHidden {
	fn is_hidden(&self) -> bool;
}

#[cfg(target_family = "unix")]
impl IsHidden for Path {
	fn is_hidden(&self) -> bool {
		match self.file_name() {
			None => false,
			Some(filename) => filename.to_string_lossy().starts_with('.'),
		}
	}
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
				log::error!("{}", e);
				false
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn check_hidden() {
		let path = Path::new("/home/user/.testfile");
		assert!(path.is_hidden())
	}

	#[test]
	fn not_hidden() {
		let path = Path::new("/home/user/testfile");
		assert!(!path.is_hidden())
	}
}
