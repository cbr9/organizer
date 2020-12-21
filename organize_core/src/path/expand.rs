use std::{
	env,
	path::{Path, PathBuf},
};
use std::env::VarError;

pub trait Expand {
	// TODO: implement for str
	fn expand_user(self) -> Result<PathBuf, VarError>
	where
		Self: Sized;
	fn expand_vars(self) -> Result<PathBuf, VarError>
	where
		Self: Sized;
}

impl<T: Into<PathBuf>> Expand for T {
	fn expand_user(self) -> Result<PathBuf, VarError> {
		let path = self.into();
		let str = path.to_string_lossy();
		if str.contains('~') {
			env::var("HOME").map(|home| str.replace("~", &home).into())
		} else {
			Ok(path)
		}
	}

	fn expand_vars(self) -> Result<PathBuf, VarError> {
		let path = self.into();
		let str = path.to_string_lossy();
		if str.contains('$') {
			let mut components = Vec::new();
			for comp in path.components() {
				let component: &Path = comp.as_ref();
				let component = component.to_string_lossy();
				if component.starts_with('$') {
					env::var(component.replace('$', "")).map(|comp| components.push(comp))?;
				} else {
					components.push(component.to_string());
				}
			}
			if str.ends_with("/") {
				components.last_mut().map(|last| last.push('/'));
			}
			Ok(components.into_iter().collect::<PathBuf>())
		} else {
			Ok(path)
		}
	}
}

#[cfg(test)]
mod tests {
	use std::env;

	use dirs::home_dir;

	use crate::utils::tests::project;

	use super::*;

	#[test]
	fn user() {
		let original = PathBuf::from("~/Documents");
		let expected = home_dir().unwrap().join("Documents");
		assert_eq!(original.expand_user().unwrap(), expected)
	}
	#[test]
	fn home() {
		let original = PathBuf::from("$HOME/Documents");
		let expected = home_dir().unwrap().join("Documents");
		assert_eq!(original.expand_vars().unwrap(), expected)
	}
	#[test]
	fn new_var() {
		env::set_var("PROJECT_DIR", project());
		let original = PathBuf::from("$PROJECT_DIR/tests");
		let expected = project().join("tests");
		assert_eq!(original.expand_vars().unwrap(), expected)
	}
	#[test]
	fn non_existing_var() {
		let var = "PROJECT_DIR_2";
		let tested = PathBuf::from(format!("${}/tests", var));
		assert!(tested.expand_vars().is_err())
	}
}
