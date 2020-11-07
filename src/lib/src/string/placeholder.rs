use std::{borrow::Cow, io, io::ErrorKind, path::Path};

use crate::string::Capitalize;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{de::Error, Deserialize, Deserializer};

lazy_static! {
	// forgive me god for this monstrosity
	static ref POTENTIAL_PH_REGEX: Regex = Regex::new(r"\{\w+(?:\.\w+)*}").unwrap();
	static ref VALID_PH_REGEX: Regex = {
		let vec = vec![
			r"\{(?:(?:path|parent)(?:\.path|\.parent)*)(?:\.filename)?(?:\.to_lowercase|\.to_uppercase|\.capitalize)?\}", // match placeholders that involve directories
			r"\{path(?:\.filename)?(?:\.extension|\.stem)?(?:\.to_lowercase|\.to_uppercase|\.capitalize)?\}", // match placeholders that involve files and start with path
			r"\{filename(?:\.extension|\.stem)?(?:\.to_lowercase|\.to_uppercase|\.capitalize)?\}", // match placeholders that involve files and start with filename
			r"\{(?:(?:extension|stem)(?:\.to_lowercase|\.to_uppercase|\.capitalize)?)\}" // match placeholders that involve files and only deal with extension/stem
		];
		let whole = vec.iter().enumerate().map(|(i, str)| {
			if i == vec.len()-1 {
				format!("(?:{})", str)
			} else {
				format!("(?:{})|", str)
			}
		}).collect::<String>();
		Regex::new(whole.as_str()).unwrap()
	};
}

// used in #[serde(deserialize_with = "..."] flags
pub fn deserialize_placeholder_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
	D: Deserializer<'de>,
{
	let v = String::deserialize(deserializer)?;
	visit_placeholder_string(v.as_str()).map_err(|e| D::Error::custom(e.to_string()))
}

// used inside Visitor impls
pub fn visit_placeholder_string(val: &str) -> Result<String, io::Error> {
	if !(POTENTIAL_PH_REGEX.is_match(val) ^ VALID_PH_REGEX.is_match(val)) {
		// if there are no matches or there are only valid ones
		Ok(val.to_string())
	} else {
		Err(io::Error::new(ErrorKind::Other, "invalid placeholder")) // if there are matches but they're invalid
	}
}

pub trait Placeholder {
	fn expand_placeholders(&self, path: &Path) -> io::Result<Cow<'_, str>>;
}

impl Placeholder for str {
	fn expand_placeholders(&self, path: &Path) -> io::Result<Cow<'_, str>> {
		if VALID_PH_REGEX.is_match(self) {
			// if the first condition is false, the second one won't be evaluated and REGEX won't be initialized
			let mut new = self.to_string();
			for span in VALID_PH_REGEX.find_iter(self) {
				let placeholders = span.as_str().trim_matches(|x| x == '{' || x == '}').split('.');
				let mut current_value = path.to_path_buf();
				for placeholder in placeholders.into_iter() {
					current_value = match placeholder {
						"path" => current_value
							.canonicalize()
							.ok()
							.ok_or_else(|| placeholder_error(placeholder, &current_value, span.as_str()))?,
						"parent" => current_value
							.parent()
							.ok_or_else(|| placeholder_error(placeholder, &current_value, span.as_str()))?
							.into(),
						"filename" => current_value
							.file_name()
							.ok_or_else(|| placeholder_error(placeholder, &current_value, span.as_str()))?
							.into(),
						"stem" => current_value
							.file_stem()
							.ok_or_else(|| placeholder_error(placeholder, &current_value, span.as_str()))?
							.into(),
						"extension" => current_value
							.extension()
							.ok_or_else(|| placeholder_error(placeholder, &current_value, span.as_str()))?
							.into(),
						"to_uppercase" => current_value.to_str().unwrap().to_uppercase().into(),
						"to_lowercase" => current_value.to_str().unwrap().to_lowercase().into(),
						"capitalize" => current_value.to_str().unwrap().to_string().capitalize().into(),
						_ => panic!("unknown placeholder"),
					}
				}
				new = new.replace(&span.as_str(), current_value.to_str().unwrap());
			}
			Ok(Cow::Owned(new.replace("//", "/")))
		} else {
			Ok(Cow::Borrowed(self))
		}
	}
}

fn placeholder_error(placeholder: &str, current_value: &Path, span: &str) -> std::io::Error {
	let message = format!(
		"tried to retrieve the {} from {}, but it does not contain it (placeholder: {})",
		placeholder,
		current_value.display(),
		span
	);
	std::io::Error::new(ErrorKind::Other, message)
}

#[cfg(test)]
pub mod tests {
	use std::{
		io::{Error, Result},
		path::PathBuf,
	};

	use crate::utils::tests::IntoResult;

	use super::*;

	#[test]
	#[should_panic]
	fn deserialize_invalid_ph_extension_name() {
		let str = "$HOME/{extension.name}";
		visit_placeholder_string(str).map(|_| ()).unwrap()
	}
	#[test]
	#[should_panic]
	fn deserialize_invalid_ph_extension_stem() {
		let str = "$HOME/{extension.stem}";
		visit_placeholder_string(str).map(|_| ()).unwrap()
	}
	#[test]
	#[should_panic]
	fn deserialize_invalid_ph_extension_extension() {
		let str = "$HOME/{extension.extension}";
		visit_placeholder_string(str).map(|_| ()).unwrap()
	}
	#[test]
	#[should_panic]
	fn deserialize_invalid_ph_stem_extension() {
		let str = "$HOME/{stem.extension}";
		visit_placeholder_string(str).map(|_| ()).unwrap()
	}
	#[test]
	#[should_panic]
	fn deserialize_invalid_ph_stem_stem() {
		let str = "$HOME/{stem.stem}";
		visit_placeholder_string(str).map(|_| ()).unwrap()
	}
	#[test]
	#[should_panic]
	fn deserialize_invalid_ph_parent_stem() {
		let str = "$HOME/{parent.stem}";
		visit_placeholder_string(str).map(|_| ()).unwrap()
	}
	#[test]
	#[should_panic]
	fn deserialize_invalid_ph_parent_extension() {
		let str = "$HOME/{parent.extension}";
		visit_placeholder_string(str).map(|_| ()).unwrap()
	}
	#[test]
	fn deserialize_valid_ph_extension() -> Result<()> {
		let str = "$HOME/{extension}";
		visit_placeholder_string(str).map(|_| ())
	}
	#[test]
	fn deserialize_valid_ph_stem() -> Result<()> {
		let str = "$HOME/{stem}";
		visit_placeholder_string(str).map(|_| ())
	}
	#[test]
	fn deserialize_valid_ph_filename() -> Result<()> {
		let str = "$HOME/{filename}";
		visit_placeholder_string(str).map(|_| ())
	}
	#[test]
	fn deserialize_valid_ph_path() -> Result<()> {
		let str = "$HOME/{path}";
		visit_placeholder_string(str).map(|_| ())
	}
	#[test]
	fn deserialize_valid_ph_parent() -> Result<()> {
		let str = "$HOME/{parent}";
		visit_placeholder_string(str).map(|_| ())
	}
	#[test]
	fn deserialize_valid_ph_extension_uppercase() -> Result<()> {
		let str = "$HOME/{extension.to_uppercase}";
		visit_placeholder_string(str).map(|_| ())
	}
	#[test]
	fn deserialize_valid_ph_stem_uppercase() -> Result<()> {
		let str = "$HOME/{stem.to_uppercase}";
		visit_placeholder_string(str).map(|_| ())
	}
	#[test]
	fn deserialize_valid_ph_filename_uppercase() -> Result<()> {
		let str = "$HOME/{filename.to_uppercase}";
		visit_placeholder_string(str).map(|_| ())
	}
	#[test]
	fn deserialize_valid_ph_path_uppercase() -> Result<()> {
		let str = "$HOME/{path.to_uppercase}";
		visit_placeholder_string(str).map(|_| ())
	}
	#[test]
	fn deserialize_valid_ph_parent_uppercase() -> Result<()> {
		let str = "$HOME/{parent.to_uppercase}";
		visit_placeholder_string(str).map(|_| ())
	}
	#[test]
	fn deserialize_valid_ph_filename_extension() -> Result<()> {
		let str = "$HOME/{filename.extension}";
		visit_placeholder_string(str).map(|_| ())
	}
	#[test]
	fn deserialize_valid_ph_filename_stem() -> Result<()> {
		let str = "$HOME/{filename.stem}";
		visit_placeholder_string(str).map(|_| ())
	}
	#[test]
	fn deserialize_valid_ph_parent_filename() -> Result<()> {
		let str = "$HOME/{parent.filename}";
		visit_placeholder_string(str).map(|_| ())
	}
	#[test]
	fn deserialize_valid_ph_parent_parent() -> Result<()> {
		let str = "$HOME/{parent.parent}";
		visit_placeholder_string(str).map(|_| ())
	}
	#[test]
	fn deserialize_valid_ph_parent_parent_parent() -> Result<()> {
		let str = "$HOME/{parent.parent.parent}";
		visit_placeholder_string(str).map(|_| ())
	}
	#[test]
	fn deserialize_valid_ph_parent_parent_parent_filename() -> Result<()> {
		let str = "$HOME/{parent.parent.parent.filename}";
		visit_placeholder_string(str).map(|_| ())
	}
	#[test]
	fn deserialize_valid_ph_path_parent() -> Result<()> {
		let str = "$HOME/{path.parent}";
		visit_placeholder_string(str).map(|_| ())
	}
	#[test]
	fn deserialize_valid_ph_path_filename() -> Result<()> {
		let str = "$HOME/{path.filename}";
		visit_placeholder_string(str).map(|_| ())
	}
	#[test]
	fn deserialize_valid_ph_path_stem() -> Result<()> {
		let str = "$HOME/{path.stem}";
		visit_placeholder_string(str).map(|_| ())
	}
	#[test]
	fn deserialize_valid_ph_path_extension() -> Result<()> {
		let str = "$HOME/{path.extension}";
		visit_placeholder_string(str).map(|_| ())
	}
	#[test]
	fn single_placeholder() -> Result<()> {
		let with_ph = "$HOME/Downloads/{parent.filename}";
		let expected = String::from("$HOME/Downloads/Documents");
		let path = Path::new("$HOME/Documents/test.pdf");
		let new_str = with_ph.expand_placeholders(path)?;
		(new_str == expected).into_result()
	}
	#[test]
	fn multiple_placeholders() -> Result<()> {
		let with_ph = "$HOME/{extension}/{parent.filename}";
		let expected = String::from("$HOME/pdf/Documents");
		let path = Path::new("$HOME/Documents/test.pdf");
		let new_str = with_ph.expand_placeholders(path)?;
		(new_str == expected).into_result()
	}
	#[test]
	fn multiple_placeholders_sentence() -> Result<()> {
		let with_ph = "To run this program, you have to change directory into $HOME/{extension}/{parent.filename}";
		let path = PathBuf::from("$HOME/Documents/test.pdf");
		let new_str = with_ph.expand_placeholders(&path)?;
		let expected = "To run this program, you have to change directory into $HOME/pdf/Documents";
		(new_str == expected).into_result()
	}
	#[test]
	fn no_placeholder() -> Result<()> {
		let tested = "/home/cabero/Documents/test.pdf";
		let dummy_path = PathBuf::from(tested);
		let new = tested.expand_placeholders(&dummy_path)?;
		match new {
			Cow::Borrowed(_) => Ok(()),
			Cow::Owned(_) => Err(Error::from(ErrorKind::Other)),
		}
	}
}
