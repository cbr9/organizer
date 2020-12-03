use std::env::VarError;
use std::{
	env,
	path::{Path, PathBuf},
};

pub trait Expand {
	// TODO: implement for str
	fn expand_user(self) -> Result<Self, VarError>
	where
		Self: Sized;
	fn expand_vars(self) -> Result<Self, VarError>
	where
		Self: Sized;
}

impl Expand for PathBuf {
	fn expand_user(self) -> Result<PathBuf, VarError> {
		let str = self.to_str().unwrap();
		if str.contains('~') {
			env::var("HOME").map(|home| str.replace("~", &home).into())
		} else {
			Ok(self)
		}
	}

	fn expand_vars(self) -> Result<PathBuf, VarError> {
		if self.to_string_lossy().contains('$') {
			let mut components = Vec::new();
			for comp in self.components() {
				let component: &Path = comp.as_ref();
				let component = component.to_string_lossy();
				if component.starts_with('$') {
					env::var(component.replace('$', "")).map(|comp| components.push(comp))?;
				} else {
					components.push(component.to_string());
				}
			}
			Ok(components.into_iter().collect::<PathBuf>())
		} else {
			Ok(self)
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
