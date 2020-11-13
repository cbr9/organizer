use std::{
	env,
	path::{Path, PathBuf},
};

pub trait Expand {
	// TODO: implement for str
	fn expand_user(self) -> Self;
	fn expand_vars(self) -> Self;
}

impl Expand for PathBuf {
	fn expand_user(self) -> PathBuf {
		let str = self.to_str().unwrap();
		if str.contains('~') {
			match env::var("HOME") {
				Ok(home) => {
					let new = str.replace("~", &home);
					new.into()
				}
				Err(e) => panic!("error: {}", e),
			}
		} else {
			self
		}
	}

	fn expand_vars(self) -> PathBuf {
		// TODO: avoid panic, return a serde error
		if self.to_string_lossy().contains('$') {
			self.components()
				.map(|component| {
					let component: &Path = component.as_ref();
					let component = component.to_string_lossy();
					if component.starts_with('$') {
						env::var(component.replace('$', ""))
							// todo: return error, don't panic
							.unwrap_or_else(|_| panic!("error: environment variable '{}' could not be found", component))
					} else {
						component.to_string()
					}
				})
				.collect::<PathBuf>()
		} else {
			self
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
	fn home() {
		let original = PathBuf::from("$HOME/Documents");
		let expected = home_dir().unwrap().join("Documents");
		assert_eq!(original.expand_vars(), expected)
	}
	#[test]
	fn new_var() {
		env::set_var("PROJECT_DIR", project());
		let original = PathBuf::from("$PROJECT_DIR/tests");
		let expected = project().join("tests");
		assert_eq!(original.expand_vars(), expected)
	}
	#[test]
	#[should_panic]
	fn non_existing_var() {
		let var = "PROJECT_DIR_2";
		let tested = PathBuf::from(format!("${}/tests", var));
		tested.expand_vars();
	}
}
