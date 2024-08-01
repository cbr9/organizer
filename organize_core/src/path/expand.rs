use anyhow::{anyhow, Context, Result};
use std::{
	env,
	ffi::{OsStr, OsString},
	iter::FromIterator,
	path::{Path, PathBuf},
};

pub trait Expand {
	fn expand_user(self) -> Result<PathBuf>
	where
		Self: Sized;
}

impl<T: Into<PathBuf>> Expand for T {
	fn expand_user(self) -> Result<PathBuf> {
		let path = self.into();
		let mut components = path.components();
		if let Some(component) = components.next() {
			if component.as_os_str() == OsStr::new("~") {
				let mut path = dirs_next::home_dir().ok_or_else(|| anyhow!("could not find home directory"))?;
				path.extend(components);
				return Ok(path);
			}
		}
		Ok(path)
	}
}

#[cfg(test)]
mod tests {
	use std::env;

	use super::*;

	#[test]
	fn invalid_tilde() {
		let original = dirs_next::home_dir().unwrap().join("Documents~");
		assert_eq!(original.clone().expand_user().unwrap(), original)
	}

	#[test]
	fn user_tilde() {
		let original = "~/Documents";
		let expected = dirs_next::home_dir().unwrap().join("Documents");
		assert_eq!(original.expand_user().unwrap(), expected)
	}
}
