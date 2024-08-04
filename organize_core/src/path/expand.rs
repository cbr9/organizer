use std::{ffi::OsStr, path::PathBuf};

pub trait Expand {
	fn expand_user(self) -> PathBuf
	where
		Self: Sized;
}

impl<T: Into<PathBuf>> Expand for T {
	fn expand_user(self) -> PathBuf {
		let path = self.into();
		let mut components = path.components();
		if let Some(component) = components.next() {
			if component.as_os_str() == OsStr::new("~") {
				let mut path = dirs::home_dir().expect("could not find home directory");
				path.extend(components);
				return path;
			}
		}
		path
	}
}

#[cfg(test)]
mod tests {

	use super::*;

	#[test]
	fn invalid_tilde() {
		let original = dirs::home_dir().unwrap().join("Documents~");
		assert_eq!(original.clone().expand_user(), original)
	}

	#[test]
	fn user_tilde() {
		let original = "~/Documents";
		let expected = dirs::home_dir().unwrap().join("Documents");
		assert_eq!(original.expand_user(), expected)
	}
}
