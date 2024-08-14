use derive_more::Deref;
use empty::Empty;
use itertools::Itertools;
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

pub trait FilterUtils {
	fn fold_vecs_with_any(&self, vecs: Vec<Vec<bool>>) -> Vec<bool> {
		let length = vecs.first().map_or(0, |v| v.len()); // Get the length of the first Vec or 0 if empty

		(0..length)
			.map(|index| vecs.iter().any(|vec| vec.get(index).copied().unwrap_or(false)))
			.collect()
	}

	fn fold_vecs_with_all(&self, vecs: Vec<Vec<bool>>) -> Vec<bool> {
		let length = vecs.first().map_or(0, |v| v.len()); // Get the length of the first Vec or 0 if empty

		(0..length)
			.map(|index| vecs.iter().all(|vec| vec.get(index).copied().unwrap_or(false)))
			.collect()
	}

	fn fold_vecs_with_none(&self, vecs: Vec<Vec<bool>>) -> Vec<bool> {
		let length = vecs.first().map_or(0, |v| v.len()); // Get the length of the first Vec or 0 if empty

		(0..length)
			.map(|index| vecs.iter().all(|vec| !vec.get(index).copied().unwrap_or(false)))
			.collect()
	}
}

impl<T> FilterUtils for T where T: AsFilter {}
impl FilterUtils for Filters {}

pub trait AsFilter {
	fn filter(&self, resources: &[&Resource]) -> Vec<bool>;
}

impl AsFilter for Filter {
	#[tracing::instrument(ret, level = "debug")]
	fn filter(&self, resources: &[&Resource]) -> Vec<bool> {
		match self {
			Filter::AllOf { filters } => {
				let results: Vec<Vec<bool>> = filters.par_iter().map(|filter| filter.filter(resources)).collect();
				self.fold_vecs_with_all(results)
			}
			Filter::AnyOf { filters } => {
				let results: Vec<Vec<bool>> = filters.par_iter().map(|filter| filter.filter(resources)).collect();
				self.fold_vecs_with_any(results)
			}
			Filter::NoneOf { filters } => {
				let results: Vec<Vec<bool>> = filters.par_iter().map(|filter| filter.filter(resources)).collect();
				self.fold_vecs_with_none(results)
			}
			Filter::Empty(filter) => filter.filter(resources),
			Filter::Extension(filter) => filter.filter(resources),
			Filter::Filename(filter) => filter.filter(resources),
			Filter::Mime(filter) => filter.filter(resources),
			Filter::Regex(filter) => filter.filter(resources),
			Filter::Script(filter) => filter.filter(resources),
			Filter::NotEmpty(filter) => filter.filter(resources).into_iter().map(|matches| !matches).collect(),
			Filter::NotExtension(filter) => filter.filter(resources).into_iter().map(|matches| !matches).collect(),
			Filter::NotFilename(filter) => filter.filter(resources).into_iter().map(|matches| !matches).collect(),
			Filter::NotMime(filter) => filter.filter(resources).into_iter().map(|matches| !matches).collect(),
			Filter::NotRegex(filter) => filter.filter(resources).into_iter().map(|matches| !matches).collect(),
			Filter::NotScript(filter) => filter.filter(resources).into_iter().map(|matches| !matches).collect(),
		}
	}
}

#[derive(Debug, Clone, Deserialize, Deref, PartialEq)]
pub struct Filters(pub(crate) Vec<Filter>);

impl Filters {
	pub fn filter(&self, resources: Vec<Resource>) -> Vec<Resource> {
		let results: Vec<Vec<bool>> = self
			.par_iter()
			.map(|filter| filter.filter(&resources.iter().collect_vec()))
			.collect();

		resources
			.into_iter()
			.zip(self.fold_vecs_with_all(results))
			.filter_map(|(res, matches)| {
				if matches {
					return Some(res);
				}
				None
			})
			.collect_vec()
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
			Filter::Extension(Extension {
				extensions: vec!["jpg".into()],
			}),
		]);
		let resources = vec![Resource::from_str("$HOME/Downloads/unsplash_image.jpg").unwrap()];
		assert_eq!(filters.filter(resources.clone()), resources);
		let resources = vec![Resource::from_str("$HOME/Downloads/unsplash_doc.pdf").unwrap()];
		assert_eq!(filters.filter(resources.clone()), vec![]);
	}
}
