use derive_more::Deref;
use empty::Empty;
use serde::Deserialize;

use extension::Extension;
use filename::Filename;

pub mod empty;
pub mod extension;
pub mod filename;
pub mod mime;
pub mod regex;

use crate::{
	config::filters::{mime::Mime, regex::Regex},
	resource::Resource,
};

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
	fn matches(&self, res: &mut Resource) -> bool;
}

impl AsFilter for Filter {
	fn matches(&self, res: &mut Resource) -> bool {
		match self {
			Filter::Regex(regex) => regex.matches(res),
			Filter::Filename(filename) => filename.matches(res),
			Filter::Extension(extension) => extension.matches(res),
			Filter::Script(script) => script.matches(res),
			Filter::Mime(mime) => mime.matches(res),
			Filter::Empty(empty) => empty.matches(res),
			Filter::Group { filters } => filters.iter().any(|f| f.matches(res)),
		}
	}
}

#[derive(Debug, Clone, Deserialize, Deref, PartialEq)]
pub struct Filters(pub(crate) Vec<Filter>);

impl AsFilter for Filters {
	fn matches(&self, res: &mut Resource) -> bool {
		self.iter().all(|filter| filter.matches(res))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::config::filters::{regex::Regex, Filter};
	use std::{convert::TryFrom, str::FromStr};

	#[test]
	fn match_all() {
		let filters = Filters(vec![
			Filter::Regex(Regex::try_from(vec![".*unsplash.*"]).unwrap()),
			Filter::Regex(Regex::try_from(vec![".*\\.jpg"]).unwrap()),
		]);
		assert!(filters.matches(&mut Resource::from_str("$HOME/Downloads/unsplash_image.jpg").unwrap()));
		assert!(!filters.matches(&mut Resource::from_str("$HOME/Downloads/unsplash_doc.pdf").unwrap()));
	}
}
