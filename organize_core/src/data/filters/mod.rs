use std::{ops::Deref, path::Path};

use serde::Deserialize;

use extension::Extension;
use filename::Filename;

mod extension;
mod filename;
#[cfg(feature = "filter_mime")]
mod mime;
mod regex;

#[cfg(feature = "filter_mime")]
use crate::data::filters::mime::MimeWrapper;
use crate::data::{actions::script::Script, filters::regex::Regex, options::apply::Apply};

#[derive(Debug, Clone, Deserialize, Eq, PartialEq)]
#[serde(rename_all(deserialize = "lowercase"))]
pub enum Filter {
	Regex(Regex),
	Filename(Filename),
	Extension(Extension),
	Script(Script),
	#[cfg(feature = "filter_mime")]
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
			#[cfg(feature = "filter_mime")]
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
		filters::{regex::Regex, Filter},
		options::apply::Apply,
	};
	use std::str::FromStr;

	#[test]
	fn match_all() {
		let filters = Filters(vec![
			Filter::Regex(Regex::from_str(".*unsplash.*").unwrap()),
			Filter::Regex(Regex::from_str(".*\\.jpg").unwrap()),
		]);
		assert!(filters.r#match("$HOME/Downloads/unsplash_image.jpg", &Apply::All));
		assert!(!filters.r#match("$HOME/Downloads/unsplash_doc.pdf", &Apply::All));
	}

	#[test]
	fn match_any() {
		let regex = Regex::from_str(".*unsplash.*").unwrap();
		let filters = Filters(vec![Filter::Regex(regex), Filter::Regex(Regex::from_str(".*\\.jpg").unwrap())]);
		assert!(filters.r#match("$HOME/Downloads/test.jpg", &Apply::Any))
	}

	#[test]
	fn match_any_of() {
		let regex = Regex::from_str(".*unsplash.*").unwrap();
		let filters = Filters(vec![
			Filter::Regex(regex),
			Filter::Regex(Regex::from_str(".*\\.pdf").unwrap()),
			Filter::Filename(Filename {
				case_sensitive: true,
				startswith: Some("random".into()),
				..Filename::default()
			}),
		]);
		let path = "$HOME/Downloads/unsplash.jpg";
		assert!(filters.r#match(&path, &Apply::AnyOf(vec![0, 1])));
		assert!(!filters.r#match(&path, &Apply::AnyOf(vec![1, 2])));
		assert!(filters.r#match(&path, &Apply::AnyOf(vec![0, 2])));
	}

	#[test]
	fn match_all_of() {
		let filters = Filters(vec![
			Filter::Regex(Regex::from_str(".*unsplash.*").unwrap()),
			Filter::Regex(Regex::from_str(".*\\.pdf").unwrap()),
			Filter::Filename(Filename {
				case_sensitive: true,
				startswith: Some("random".into()),
				..Filename::default()
			}),
			Filter::Regex(Regex::from_str(".*\\.jpg").unwrap()),
		]);
		let path = "$HOME/Downloads/unsplash.jpg";
		assert!(!filters.r#match(&path, &Apply::AllOf(vec![0, 1])));
		assert!(!filters.r#match(&path, &Apply::AllOf(vec![1, 2])));
		assert!(!filters.r#match(&path, &Apply::AllOf(vec![1, 3])));
		assert!(!filters.r#match(&path, &Apply::AllOf(vec![2, 3])));
		assert!(!filters.r#match(&path, &Apply::AllOf(vec![0, 2])));
		assert!(filters.r#match(&path, &Apply::AllOf(vec![0, 3])));
	}
}
