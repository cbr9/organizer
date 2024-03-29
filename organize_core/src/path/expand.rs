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
	fn expand_vars(self) -> Result<PathBuf>
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

	fn expand_vars(self) -> Result<PathBuf> {
		let path = self.into();
		let str = path.to_string_lossy();
		if str.contains('$') {
			let mut new_components = Vec::with_capacity(path.components().count());
			for comp in path.components() {
				let component_path: &Path = comp.as_ref();
				let component_str = component_path.to_string_lossy();
				if component_str.starts_with('$') {
					let key = component_str.replace('$', "");
					let value = env::var_os(&key).with_context(|| format!("could not find ${} environment variable", key))?;
					new_components.push(value);
				} else {
					let str = OsString::from(component_path);
					new_components.push(str);
				}
			}
			if str.ends_with('/') {
				if let Some(last) = new_components.last_mut() {
					last.push("/")
				}
			}
			Ok(PathBuf::from_iter(new_components))
		} else {
			Ok(path)
		}
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
	#[test]
	fn home() {
		let original = "$HOME/Documents";
		let expected = dirs_next::home_dir().unwrap().join("Documents");
		assert_eq!(original.expand_vars().unwrap(), expected)
	}
	#[test]
	fn new_var() {
		let var = "DIR";
		env::set_var(var, "DIR");
		let original = format!("${}/tests", var);
		let expected = PathBuf::from("DIR").join("tests");
		assert_eq!(original.expand_vars().unwrap(), expected);
		env::remove_var(var);
	}
	#[test]
	fn non_existing_var() {
		let tested = "$NON_EXISTING_VAR/tests";
		assert!(tested.expand_vars().is_err())
	}
}
