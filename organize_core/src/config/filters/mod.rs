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

use crate::{
	config::filters::{mime::Mime, regex::Regex},
	resource::Resource,
};

use super::actions::script::Script;

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Filter {
	Regex(Regex),
	Empty(Empty),
	Filename(Filename),
	Extension(Extension),
	Script(Script),
	Mime(Mime),
	#[serde(rename = "!regex")]
	NotRegex(Regex),
	#[serde(rename = "!empty")]
	NotEmpty(Empty),
	#[serde(rename = "!filename")]
	NotFilename(Filename),
	#[serde(rename = "!extension")]
	NotExtension(Extension),
	#[serde(rename = "!script")]
	NotScript(Script),
	#[serde(rename = "!mime")]
	NotMime(Mime),
	AnyOf {
		filters: Vec<Filter>,
	},
	AllOf {
		filters: Vec<Filter>,
	},
	NoneOf {
		filters: Vec<Filter>,
	},
}

pub trait AsFilter {
	fn matches(&self, res: &Resource) -> bool;
}

impl AsFilter for Filter {
	fn matches(&self, res: &Resource) -> bool {
		match self {
			Filter::AllOf { filters } => filters.par_iter().all(|f| f.matches(res)),
			Filter::AnyOf { filters } => filters.par_iter().any(|f| f.matches(res)),
			Filter::NoneOf { filters } => filters.par_iter().all(|f| !f.matches(res)),
			Filter::Empty(filter) => filter.matches(res),
			Filter::Extension(filter) => filter.matches(res),
			Filter::Filename(filter) => filter.matches(res),
			Filter::Mime(filter) => filter.matches(res),
			Filter::Regex(filter) => filter.matches(res),
			Filter::Script(filter) => filter.matches(res),
			Filter::NotEmpty(filter) => !filter.matches(res),
			Filter::NotExtension(filter) => !filter.matches(res),
			Filter::NotFilename(filter) => !filter.matches(res),
			Filter::NotMime(filter) => !filter.matches(res),
			Filter::NotRegex(filter) => !filter.matches(res),
			Filter::NotScript(filter) => !filter.matches(res),
		}
	}
}

#[derive(Debug, Clone, Deserialize, Deref, PartialEq)]
pub struct Filters(pub(crate) Vec<Filter>);

impl AsFilter for Filters {
	fn matches(&self, res: &Resource) -> bool {
		self.par_iter().all(|filter| filter.matches(res))
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
		assert!(filters.matches(&Resource::from_str("$HOME/Downloads/unsplash_image.jpg").unwrap()));
		assert!(!filters.matches(&Resource::from_str("$HOME/Downloads/unsplash_doc.pdf").unwrap()));
	}
}
