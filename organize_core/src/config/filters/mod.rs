use std::{ops::Deref, path::Path};

use serde::Deserialize;

use extension::Extension;
use filename::Filename;

mod extension;
mod filename;
mod regex;
pub use self::regex::*;
use crate::config::{Apply, Script};
pub use extension::*;
pub use filename::*;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all(deserialize = "lowercase"))]
pub enum Filter {
	Regex(Regex),
	Filename(Filename),
	Extension(Extension),
	Script(Script),
}

pub trait AsFilter {
	fn matches(&self, path: &Path) -> bool;
}

impl AsFilter for Filter {
	fn matches(&self, path: &Path) -> bool {
		match self {
			Filter::Regex(regex) => regex.matches(path),
			Filter::Filename(filename) => filename.matches(path),
			Filter::Extension(extension) => extension.matches(path),
			Filter::Script(script) => script.matches(path),
		}
	}
}

#[derive(Debug, Clone, Deserialize)]
pub struct Filters {
	inner: Vec<Filter>,
}

impl Deref for Filters {
	type Target = Vec<Filter>;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl Filters {
	pub fn r#match<T, A>(&self, path: T, apply: A) -> bool
	where
		T: AsRef<Path>,
		A: AsRef<Apply>,
	{
		let temp_files = ["crdownload", "part"];
		if temp_files.contains(&&*path.as_ref().extension().unwrap_or_default().to_string_lossy()) {
			return false;
		}
		match apply.as_ref() {
			Apply::All => self.iter().all(|filter| filter.matches(path.as_ref())),
			Apply::Any => self.iter().any(|filter| filter.matches(path.as_ref())),
			Apply::AllOf(filters) => self
				.iter()
				.enumerate()
				.filter(|(i, _)| filters.contains(i))
				.all(|(_, filter)| filter.matches(path.as_ref())),
			Apply::AnyOf(filters) => self
				.iter()
				.enumerate()
				.filter(|(i, _)| filters.contains(i))
				.any(|(_, filter)| filter.matches(path.as_ref())),
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::config::{Apply, Filter, Filters, Regex};
	use std::{path::PathBuf, str::FromStr};

	#[test]
	fn match_partial_file() {
		let regex = Regex::from_str(".*").unwrap();
		let filters = Filters {
			inner: vec![Filter::Regex(regex)],
		};
		let path = PathBuf::from("$HOME/Downloads/test.crdownload");
		assert!(!filters.r#match(&path, Apply::All))
	}
}
