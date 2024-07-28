use anyhow::{anyhow, bail, Context, Result};
use std::{collections::HashMap, ffi::OsString, path::Path, str::FromStr};

use crate::{
	fsa::{Fsa, Transition},
	string::Capitalize,
	transition, transitions,
};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{de::Error, Deserialize, Deserializer};

lazy_static! {
	static ref POTENTIAL_PH_REGEX: Regex = Regex::new(r"\{\w+(?:\.\w+)*}").unwrap(); // a panic here indicates a compile-time bug
	static ref PLACEHOLDER_TO_ALIASES: HashMap<Placeholder, &'static str> =  HashMap::from([
			(Placeholder::Path, "path"),
			(Placeholder::Parent, "parent"),
			(Placeholder::Filename, "filename"),
			(Placeholder::Extension, "extension"),
			(Placeholder::Stem, "stem"),
			(Placeholder::ToUpperCase, "to_uppercase"),
			(Placeholder::ToLowerCase, "to_lowercase"),
			(Placeholder::Capitalize, "capitalize"),
		]);

	static ref PLACEHOLDER_ALIASES: Vec<&'static str> = vec![
		PLACEHOLDER_TO_ALIASES[&Placeholder::Path],
		PLACEHOLDER_TO_ALIASES[&Placeholder::Parent],
		PLACEHOLDER_TO_ALIASES[&Placeholder::Filename],
		PLACEHOLDER_TO_ALIASES[&Placeholder::Stem],
		PLACEHOLDER_TO_ALIASES[&Placeholder::Extension],
		PLACEHOLDER_TO_ALIASES[&Placeholder::ToLowerCase],
		PLACEHOLDER_TO_ALIASES[&Placeholder::ToUpperCase],
		PLACEHOLDER_TO_ALIASES[&Placeholder::Capitalize]
	];

	static ref PARSER: Fsa<'static, u8> = Fsa::new(
		&[0, 1, 2, 3, 4, 5],
		&PLACEHOLDER_ALIASES,
		0,
		&[0, 1, 2, 3, 4, 5],
		transitions![
			// On <string>, on <int>, go to  <int>
			(PLACEHOLDER_TO_ALIASES[&Placeholder::Path], 0) => 0,
			(PLACEHOLDER_TO_ALIASES[&Placeholder::Parent], 0) => 1,
			(PLACEHOLDER_TO_ALIASES[&Placeholder::Filename], 0) => 2,
			(PLACEHOLDER_TO_ALIASES[&Placeholder::Stem], 0) => 4,
			(PLACEHOLDER_TO_ALIASES[&Placeholder::Extension], 0) => 4,
			(PLACEHOLDER_TO_ALIASES[&Placeholder::ToLowerCase], 0) => 3,
			(PLACEHOLDER_TO_ALIASES[&Placeholder::ToUpperCase], 0) => 3,
			(PLACEHOLDER_TO_ALIASES[&Placeholder::Capitalize], 0) => 3,
			// --------------------
			(PLACEHOLDER_TO_ALIASES[&Placeholder::Filename], 1) => 5,
			(PLACEHOLDER_TO_ALIASES[&Placeholder::Path], 1) => 1,
			(PLACEHOLDER_TO_ALIASES[&Placeholder::Parent], 1) => 1,
			(PLACEHOLDER_TO_ALIASES[&Placeholder::ToLowerCase], 1) => 3,
			(PLACEHOLDER_TO_ALIASES[&Placeholder::ToUpperCase], 1) => 3,
			(PLACEHOLDER_TO_ALIASES[&Placeholder::Capitalize], 1) => 3,
			// --------------------
			(PLACEHOLDER_TO_ALIASES[&Placeholder::Stem], 2) => 4,
			(PLACEHOLDER_TO_ALIASES[&Placeholder::Extension], 2) => 4,
			(PLACEHOLDER_TO_ALIASES[&Placeholder::ToLowerCase], 2) => 3,
			(PLACEHOLDER_TO_ALIASES[&Placeholder::ToUpperCase], 2) => 3,
			(PLACEHOLDER_TO_ALIASES[&Placeholder::Capitalize], 2) => 3,
			// --------------------
			(PLACEHOLDER_TO_ALIASES[&Placeholder::ToLowerCase], 4) => 3,
			(PLACEHOLDER_TO_ALIASES[&Placeholder::ToUpperCase], 4) => 3,
			(PLACEHOLDER_TO_ALIASES[&Placeholder::Capitalize], 4) => 3,
			// --------------------
			(PLACEHOLDER_TO_ALIASES[&Placeholder::ToUpperCase], 5) => 3,
			(PLACEHOLDER_TO_ALIASES[&Placeholder::ToLowerCase], 5) => 3,
			(PLACEHOLDER_TO_ALIASES[&Placeholder::Capitalize], 5) => 3
		]
	);

}

// used in #[serde(deserialize_with = "...] flags
pub fn deserialize_placeholder_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
	D: Deserializer<'de>,
{
	let v = String::deserialize(deserializer)?;
	visit_placeholder_string(v.as_str()).map_err(D::Error::custom)
}

// used inside Visitor impls
pub fn visit_placeholder_string(val: &str) -> Result<String> {
	POTENTIAL_PH_REGEX.find_iter(val).try_for_each(|capture| {
		let pieces = capture.as_str().trim_matches(|pat| pat == '{' || pat == '}').split('.');
		match PARSER.accepts(pieces) {
			true => Ok(()),
			false => bail!("Invalid placeholder"),
		}
	})?;

	Ok(val.to_string())
}

pub trait ExpandPlaceholder {
	fn expand_placeholders<P: AsRef<Path>>(self, path: P) -> Result<OsString>;
}

#[derive(PartialEq, Eq, Hash, Copy, Clone)]
enum PathPlaceholder {
	RelativePath,
	AbsolutePath,
	Parent,
	Filename,
	Extension,
	Stem,
}

#[derive(PartialEq, Eq, Hash, Copy, Clone)]
enum StringPlaceholder {
	Lowercase,
	Uppercase,
	Capitalize,
}

#[derive(PartialEq, Eq, Hash, Copy, Clone)]
enum Placeholder {
	Path,
	Parent,
	Filename,
	Extension,
	Stem,
	ToLowerCase,
	ToUpperCase,
	Capitalize,
}

impl FromStr for Placeholder {
	type Err = anyhow::Error;
	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		for (key, alias) in PLACEHOLDER_TO_ALIASES.iter() {
			if alias == &s {
				return Ok(*key);
			}
		}

		Err(anyhow!("Unknown placeholder"))
	}
}

impl Placeholder {
	fn expand<P: AsRef<Path>>(self, path: P) -> Result<OsString> {
		let path = path.as_ref();
		match self {
			Self::Path => path
				.canonicalize()
				.with_context(|| format!("could not retrieve the absolute path of {}", path.display()))
				.map(OsString::from),
			Self::Parent => path
				.parent()
				.ok_or_else(|| anyhow!("{} does not have a parent directory", path.display()))
				.map(OsString::from),
			Self::Filename => path
				.file_name()
				.ok_or_else(|| anyhow!("{} does not have a filename", path.display()))
				.map(OsString::from),
			Self::Stem => path
				.file_stem()
				.ok_or_else(|| anyhow!("{} does not have a filestem", path.display()))
				.map(OsString::from),
			Self::Extension => path
				.extension()
				.ok_or_else(|| anyhow!("{} does not have an extension", path.display()))
				.map(OsString::from),
			Self::ToLowerCase => Ok(path.to_string_lossy().to_lowercase().into()),
			Self::ToUpperCase => Ok(path.to_string_lossy().to_uppercase().into()),
			Self::Capitalize => Ok(path.to_string_lossy().capitalize().into()),
		}
	}
}

impl<T: AsRef<str>> ExpandPlaceholder for T {
	fn expand_placeholders<P: AsRef<Path>>(self, path: P) -> Result<OsString> {
		let mut new = self.as_ref().to_string();
		let original = new.clone();

		for span in POTENTIAL_PH_REGEX.find_iter(&original) {
			let span = span.as_str();
			let mut current = path.as_ref().to_path_buf().into_os_string();
			let placeholders: Vec<Placeholder> = span
				.trim_matches(|x| x == '{' || x == '}')
				.split('.')
				.map(Placeholder::from_str)
				.collect::<Result<Vec<Placeholder>, _>>()?;

			for placeholder in placeholders.into_iter() {
				current = placeholder.expand(&current)?;
			}

			new = new.replace(span, &current.to_string_lossy());
		}

		Ok(new.into())
	}
}

#[cfg(test)]
pub mod tests {
	use std::path::PathBuf;

	use super::*;
	#[test]
	fn deserialize_invalid_ph_non_symbol() {
		let str = "$HOME/{extension.name}";
		assert!(visit_placeholder_string(str).is_err())
	}

	#[test]
	fn deserialize_invalid_ph_extension_name() {
		let str = "$HOME/{extension.filename}";
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
	fn deserialize_invalid_ph_parent_filename_stem() {
		let str = "$HOME/{parent.filename.stem}";
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
		let path = Path::new("$HOME/Documents/test.pdf");
		let expected = OsString::from("$HOME/Downloads/Documents");
		let new_str = with_ph.expand_placeholders(path).unwrap();
		assert_eq!(new_str, expected)
	}
	#[test]
	fn multiple_placeholders() {
		let with_ph = "$HOME/{extension}/{parent.filename}";
		let expected = OsString::from("$HOME/pdf/Documents");
		let path = Path::new("$HOME/Documents/test.pdf");
		let new_str = with_ph.expand_placeholders(path).unwrap();
		assert_eq!(new_str, expected)
	}
	#[test]
	fn multiple_placeholders_sentence() {
		let with_ph = "To run this program, you have to change directory into $HOME/{extension}/{parent.filename}";
		let path = PathBuf::from("$HOME/Documents/test.pdf");
		let new_str = with_ph.expand_placeholders(path).unwrap();
		let expected = "To run this program, you have to change directory into $HOME/pdf/Documents";
		assert_eq!(new_str, expected)
	}
	#[test]
	fn no_placeholder() {
		let tested = "/home/cabero/Documents/test.pdf";
		let dummy_path = PathBuf::from(tested);
		let new = tested.expand_placeholders(dummy_path).unwrap();
		assert_eq!(new, tested)
	}
}
