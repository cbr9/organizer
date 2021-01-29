use std::{ops::Deref, path::Path};

use serde::Deserialize;

use extension::Extension;
use filename::Filename;

mod extension;
mod filename;
mod mime;
mod regex;

use crate::data::{
	config::{
		actions::script::Script,
		filters::{mime::MimeWrapper, regex::Regex},
	},
	options::apply::Apply,
};

#[derive(Debug, Clone, Deserialize, Eq, PartialEq)]
#[serde(rename_all(deserialize = "lowercase"))]
pub enum Filter {
	Regex(Regex),
	Filename(Filename),
	Extension(Extension),
	Script(Script),
	Mime(MimeWrapper),
}

pub trait AsFilter {
	fn matches<T: AsRef<Path>>(&self, path: T) -> bool;
}

impl AsFilter for Filter {
	fn matches<T: AsRef<Path>>(&self, path: T) -> bool {
		match self {
			Filter::Regex(regex) => regex.matches(path),
			Filter::Filename(filename) => filename.matches(path),
			Filter::Extension(extension) => extension.matches(path),
			Filter::Script(script) => script.matches(path),
			Filter::Mime(mime) => mime.matches(path),
		}
	}
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq)]
pub struct Filters(pub(crate) Vec<Filter>);

impl Deref for Filters {
	type Target = Vec<Filter>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl Filters {
	pub fn r#match<T: AsRef<Path>>(&self, path: T, apply: &Apply) -> bool {
		match path.as_ref().extension() {
			None => {}
			Some(extension) => {
				let extension = extension.to_string_lossy();
				let temp_files = ["crdownload" /* chrome download */, "part"];
				if temp_files.iter().any(|temp| temp == &extension) {
					return false;
				}
			}
		}
		match apply {
			Apply::All => self.iter().all(|filter| filter.matches(&path)),
			Apply::Any => self.iter().any(|filter| filter.matches(&path)),
			Apply::AllOf(filters) => self
				.iter()
				.enumerate()
				.filter(|(i, _)| filters.contains(i))
				.all(|(_, filter)| filter.matches(&path)),
			Apply::AnyOf(filters) => self
				.iter()
				.enumerate()
				.filter(|(i, _)| filters.contains(i))
				.any(|(_, filter)| filter.matches(&path)),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::data::{
		config::filters::{regex::Regex, Filter},
		options::apply::Apply,
	};
	use std::{path::PathBuf, str::FromStr};

	#[test]
	fn do_not_match_partial_file() {
		let regex = Regex::from_str(".*").unwrap();
		let filters = Filters(vec![Filter::Regex(regex)]);
		let path = PathBuf::from("$HOME/Downloads/test.crdownload");
		assert!(!filters.r#match(&path, &Apply::All))
	}
}
