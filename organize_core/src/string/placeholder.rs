use anyhow::{anyhow, bail, Context, Result};
use std::path::{Path};

use crate::string::Capitalize;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{de::Error, Deserialize, Deserializer};

lazy_static! {
	// forgive me god for this monstrosity
	static ref POTENTIAL_PH_REGEX: Regex = Regex::new(r"\{\w+(?:\.\w+)*}").unwrap(); // a panic here indicates a compile-time bug
	static ref VALID_PH_REGEX: Regex = {
		let vec = vec![
			r"\{(?:(?:path|parent)(?:\.path|\.parent)*)(?:\.filename)?(?:\.to_lowercase|\.to_uppercase|\.capitalize)?\}", // match placeholders that involve directories
			r"\{path(?:\.filename)?(?:\.extension|\.stem)?(?:\.to_lowercase|\.to_uppercase|\.capitalize)?\}", // match placeholders that involve files and start with path
			r"\{filename(?:\.extension|\.stem)?(?:\.to_lowercase|\.to_uppercase|\.capitalize)?\}", // match placeholders that involve files and start with filename
			r"\{(?:(?:extension|stem)(?:\.to_lowercase|\.to_uppercase|\.capitalize)?)\}" // match placeholders that involve files and only deal with extension/stem
		];
		let whole = vec.iter().enumerate().map(|(i, str)| {
			// stick together the regex in `vec`
			if i == vec.len()-1 {
				format!("(?:{})", str)
			} else {
				format!("(?:{})|", str)
			}
		}).collect::<String>();
		Regex::new(whole.as_str()).unwrap() // a panic here indicates a compile-time bug
	};
}

// used in #[serde(deserialize_with = "..."] flags
pub fn deserialize_placeholder_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
	D: Deserializer<'de>,
{
	let v = String::deserialize(deserializer)?;
	visit_placeholder_string(v.as_str()).map_err(D::Error::custom)
}

// used inside Visitor impls
pub fn visit_placeholder_string(val: &str) -> Result<String> {
	if !(POTENTIAL_PH_REGEX.is_match(val) ^ VALID_PH_REGEX.is_match(val)) {
		// if there are no matches or there are only valid ones
		Ok(val.to_string())
	} else {
		bail!("invalid placeholder")
	}
}

pub trait Placeholder {
	fn expand_placeholders<P: AsRef<Path>>(self, path: P) -> Result<String>;
}

impl<T: AsRef<str>> Placeholder for T {
	fn expand_placeholders<P: AsRef<Path>>(self, path: P) -> Result<String> {
		let str = self.as_ref();
		if VALID_PH_REGEX.is_match(str) {
			// if the first condition is false, the second one won't be evaluated and REGEX won't be initialized
			let mut new = str.to_string();
			for span in VALID_PH_REGEX.find_iter(str) {
				let placeholders = span.as_str().trim_matches(|x| x == '{' || x == '}').split('.'); // split a placeholder like {path.filename.extension} into [path, filename, extension]
				let mut current_value = path.as_ref().to_path_buf();
				for placeholder in placeholders.into_iter() {
					current_value = match placeholder {
						"path" => current_value
							.canonicalize()
							.with_context(|| format!("could not retrieve the absolute path of {}", current_value.display()))?,
						"parent" => current_value
							.parent()
							.ok_or_else(|| anyhow!("{} does not have a parent directory", current_value.display()))?
							.into(),
						"filename" => current_value
							.file_name()
							.ok_or_else(|| anyhow!("{} does not have a filename", current_value.display()))?
							.into(),
						"stem" => current_value
							.file_stem()
							.ok_or_else(|| anyhow!("{} does not have a filestem", current_value.display()))?
							.into(),
						"extension" => current_value
							.extension()
							.ok_or_else(|| anyhow!("{} does not have an extension", current_value.display()))?
							.into(),
						"to_uppercase" => current_value.to_string_lossy().to_uppercase().into(),
						"to_lowercase" => current_value.to_string_lossy().to_lowercase().into(),
						"capitalize" => current_value.to_string_lossy().capitalize().into(),
						placeholder => bail!("unknown placeholder {}", placeholder),
					}
				}
				new = new.replace(&span.as_str(), &*current_value.to_string_lossy());
			}
			Ok(new.replace("//", "/"))
		} else {
			Ok(self.as_ref().to_string())
		}
	}
}

#[cfg(test)]
pub mod tests {
	use std::path::PathBuf;

	use super::*;

	#[test]
	fn deserialize_invalid_ph_extension_name() {
		let str = "$HOME/{extension.name}";
		assert!(visit_placeholder_string(str).is_err())
	}
	#[test]
	fn deserialize_invalid_ph_extension_stem() {
		let str = "$HOME/{extension.stem}";
		assert!(visit_placeholder_string(str).is_err())
	}
	#[test]
	fn deserialize_invalid_ph_extension_extension() {
		let str = "$HOME/{extension.extension}";
		assert!(visit_placeholder_string(str).is_err())
	}
	#[test]
	fn deserialize_invalid_ph_stem_extension() {
		let str = "$HOME/{stem.extension}";
		assert!(visit_placeholder_string(str).is_err())
	}
	#[test]
	fn deserialize_invalid_ph_stem_stem() {
		let str = "$HOME/{stem.stem}";
		assert!(visit_placeholder_string(str).is_err())
	}
	#[test]
	fn deserialize_invalid_ph_parent_stem() {
		let str = "$HOME/{parent.stem}";
		assert!(visit_placeholder_string(str).is_err())
	}
	#[test]
	fn deserialize_invalid_ph_parent_extension() {
		let str = "$HOME/{parent.extension}";
		assert!(visit_placeholder_string(str).is_err())
	}
	#[test]
	fn deserialize_valid_ph_extension() {
		let str = "$HOME/{extension}";
		assert!(visit_placeholder_string(str).is_ok())
	}
	#[test]
	fn deserialize_valid_ph_stem() {
		let str = "$HOME/{stem}";
		assert!(visit_placeholder_string(str).is_ok())
	}
	#[test]
	fn deserialize_valid_ph_filename() {
		let str = "$HOME/{filename}";
		assert!(visit_placeholder_string(str).is_ok())
	}
	#[test]
	fn deserialize_valid_ph_path() {
		let str = "$HOME/{path}";
		assert!(visit_placeholder_string(str).is_ok())
	}
	#[test]
	fn deserialize_valid_ph_parent() {
		let str = "$HOME/{parent}";
		assert!(visit_placeholder_string(str).is_ok())
	}
	#[test]
	fn deserialize_valid_ph_extension_uppercase() {
		let str = "$HOME/{extension.to_uppercase}";
		assert!(visit_placeholder_string(str).is_ok())
	}
	#[test]
	fn deserialize_valid_ph_stem_uppercase() {
		let str = "$HOME/{stem.to_uppercase}";
		assert!(visit_placeholder_string(str).is_ok())
	}
	#[test]
	fn deserialize_valid_ph_filename_uppercase() {
		let str = "$HOME/{filename.to_uppercase}";
		assert!(visit_placeholder_string(str).is_ok())
	}
	#[test]
	fn deserialize_valid_ph_path_uppercase() {
		let str = "$HOME/{path.to_uppercase}";
		assert!(visit_placeholder_string(str).is_ok())
	}
	#[test]
	fn deserialize_valid_ph_parent_uppercase() {
		let str = "$HOME/{parent.to_uppercase}";
		assert!(visit_placeholder_string(str).is_ok())
	}
	#[test]
	fn deserialize_valid_ph_filename_extension() {
		let str = "$HOME/{filename.extension}";
		assert!(visit_placeholder_string(str).is_ok())
	}
	#[test]
	fn deserialize_valid_ph_filename_stem() {
		let str = "$HOME/{filename.stem}";
		assert!(visit_placeholder_string(str).is_ok())
	}
	#[test]
	fn deserialize_valid_ph_parent_filename() {
		let str = "$HOME/{parent.filename}";
		assert!(visit_placeholder_string(str).is_ok())
	}
	#[test]
	fn deserialize_valid_ph_parent_parent() {
		let str = "$HOME/{parent.parent}";
		assert!(visit_placeholder_string(str).is_ok())
	}
	#[test]
	fn deserialize_valid_ph_parent_parent_parent() {
		let str = "$HOME/{parent.parent.parent}";
		assert!(visit_placeholder_string(str).is_ok())
	}
	#[test]
	fn deserialize_valid_ph_parent_parent_parent_filename() {
		let str = "$HOME/{parent.parent.parent.filename}";
		assert!(visit_placeholder_string(str).is_ok())
	}
	#[test]
	fn deserialize_valid_ph_path_parent() {
		let str = "$HOME/{path.parent}";
		assert!(visit_placeholder_string(str).is_ok())
	}
	#[test]
	fn deserialize_valid_ph_path_filename() {
		let str = "$HOME/{path.filename}";
		assert!(visit_placeholder_string(str).is_ok())
	}
	#[test]
	fn deserialize_valid_ph_path_stem() {
		let str = "$HOME/{path.stem}";
		assert!(visit_placeholder_string(str).is_ok())
	}
	#[test]
	fn deserialize_valid_ph_path_extension() {
		let str = "$HOME/{path.extension}";
		assert!(visit_placeholder_string(str).is_ok())
	}
	#[test]
	fn single_placeholder() {
		let with_ph = "$HOME/Downloads/{parent.filename}";
		let expected = String::from("$HOME/Downloads/Documents");
		let path = Path::new("$HOME/Documents/test.pdf");
		let new_str = with_ph.expand_placeholders(path).unwrap();
		assert_eq!(new_str, expected)
	}
	#[test]
	fn multiple_placeholders() {
		let with_ph = "$HOME/{extension}/{parent.filename}";
		let expected = String::from("$HOME/pdf/Documents");
		let path = Path::new("$HOME/Documents/test.pdf");
		let new_str = with_ph.expand_placeholders(path).unwrap();
		assert_eq!(new_str, expected)
	}
	#[test]
	fn multiple_placeholders_sentence() {
		let with_ph = "To run this program, you have to change directory into $HOME/{extension}/{parent.filename}";
		let path = PathBuf::from("$HOME/Documents/test.pdf");
		let new_str = with_ph.expand_placeholders(&path).unwrap();
		let expected = "To run this program, you have to change directory into $HOME/pdf/Documents";
		assert_eq!(new_str, expected)
	}
	#[test]
	fn no_placeholder() {
		let tested = "/home/cabero/Documents/test.pdf";
		let dummy_path = PathBuf::from(tested);
		let new = tested.expand_placeholders(&dummy_path).unwrap();
		assert_eq!(new, tested)
	}
}
