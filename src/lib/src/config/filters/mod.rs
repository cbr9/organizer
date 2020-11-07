use std::{ops::Deref, path::Path};

use serde::Deserialize;

use extension::Extension;
use filename::Filename;

mod extension;
mod filename;
mod regex;
pub use extension::*;
pub use filename::*;
pub use self::regex::*;
use crate::config::{Script, Apply};

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
pub struct Filters(Vec<Filter>);

impl Deref for Filters {
	type Target = Vec<Filter>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl Filters {
	pub fn r#match<T, A>(&self, path: T, apply: A) -> bool
	where
		T: AsRef<Path>,
		A: AsRef<Apply>,
	{
		match apply.as_ref() {
			Apply::All => self.iter().all(|filter| filter.matches(path.as_ref())),
			Apply::Any => self.iter().any(|filter| filter.matches(path.as_ref())),
			Apply::Select(filters) => self
				.iter()
				.enumerate()
				.filter(|(i, _)| filters.contains(i))
				.all(|(_, filter)| filter.matches(path.as_ref())),
		}
	}
}
