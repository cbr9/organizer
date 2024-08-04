use std::path::Path;

use derive_more::Deref;
use empty::Empty;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::Deserialize;

use extension::Extension;
use filename::Filename;

pub mod empty;
pub mod extension;
pub mod filename;
pub mod mime;
pub mod regex;

use crate::config::filters::{mime::Mime, regex::Regex};

use super::actions::script::Script;

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all(deserialize = "lowercase"))]
pub enum Filter {
	Regex(Regex),
	Empty(Empty),
	Filename(Filename),
	Extension(Extension),
	Script(Script),
	Mime(Mime),
	Group { filters: Vec<Filter> },
}

pub trait AsFilter {
	fn matches<T: AsRef<Path>>(&self, path: T) -> bool;
}

impl AsFilter for Filter {
	fn matches<T: AsRef<Path>>(&self, path: T) -> bool {
		let path = path.as_ref();
		match self {
			Filter::Regex(regex) => regex.matches(path),
			Filter::Filename(filename) => filename.matches(path),
			Filter::Extension(extension) => extension.matches(path),
			Filter::Script(script) => script.matches(path),
			Filter::Mime(mime) => mime.matches(path),
			Filter::Empty(empty) => empty.matches(path),
			Filter::Group { filters } => filters.par_iter().any(|f| f.matches(path)),
		}
	}
}

#[derive(Debug, Clone, Deserialize, Deref, PartialEq)]
pub struct Filters(pub(crate) Vec<Filter>);

impl AsFilter for Filters {
	fn matches<T: AsRef<Path>>(&self, path: T) -> bool {
		let path = path.as_ref();
		self.par_iter().all(|filter| filter.matches(path))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::config::filters::{regex::Regex, Filter};
	use std::convert::TryFrom;

	#[test]
	fn match_all() {
		let filters = Filters(vec![
			Filter::Regex(Regex::try_from(vec![".*unsplash.*"]).unwrap()),
			Filter::Regex(Regex::try_from(vec![".*\\.jpg"]).unwrap()),
		]);
		assert!(filters.matches("$HOME/Downloads/unsplash_image.jpg"));
		assert!(!filters.matches("$HOME/Downloads/unsplash_doc.pdf"));
	}
}
