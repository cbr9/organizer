use crate::{config::filters::AsFilter, resource::Resource};
use itertools::Itertools;
use mime::FromStrError;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Deserializer};
use std::{convert::TryFrom, ops::Deref, str::FromStr};

impl FromStr for Mime {
	type Err = FromStrError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Self::try_from(vec![s])
	}
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mime {
	#[serde(deserialize_with = "deserialize_mimetypes")]
	types: Vec<MimeInternal>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MimeInternal {
	mime: mime::Mime,
	negate: bool,
}

impl Deref for MimeInternal {
	type Target = mime::Mime;

	fn deref(&self) -> &Self::Target {
		&self.mime
	}
}

fn deserialize_mimetypes<'de, D>(deserializer: D) -> Result<Vec<MimeInternal>, D::Error>
where
	D: Deserializer<'de>,
{
	// Deserialize as a Vec<String>
	let patterns_str: Vec<String> = Vec::deserialize(deserializer)?;
	Mime::try_from(patterns_str).map(|o| o.types).map_err(serde::de::Error::custom)
}

impl<T: ToString> TryFrom<Vec<T>> for Mime {
	type Error = FromStrError;

	fn try_from(value: Vec<T>) -> Result<Self, Self::Error> {
		Ok(Self {
			types: value
				.iter()
				.map(|s| {
					let mut str = s.to_string();
					let mut negate = false;
					if str.starts_with('!') {
						negate = true;
						str = str.replacen('!', "", 1);
					}
					let mime = mime::Mime::from_str(&str.to_string()).unwrap();
					MimeInternal { mime, negate }
				})
				.collect_vec(),
		})
	}
}

impl AsFilter for Mime {
	#[tracing::instrument(ret, level = "debug")]
	fn filter(&self, resources: &[&Resource]) -> Vec<bool> {
		resources
			.into_par_iter()
			.map(|res| {
				let guess = mime_guess::from_path(&res.path).first_or_octet_stream();
				self.types.iter().any(|mime| {
					let mut matches = match (mime.type_(), mime.subtype()) {
						(mime::STAR, subtype) => subtype == guess.subtype(),
						(type_, mime::STAR) => type_ == guess.type_(),
						(type_, subtype) => type_ == guess.type_() && subtype == guess.subtype(),
					};
					if mime.negate {
						matches = !matches;
					}
					matches
				})
			})
			.collect()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn test_match_negative() {
		let types = Mime::try_from(vec!["!image/*", "audio/*"]).unwrap();
		let img = Resource::from_str("test.jpg").unwrap();
		let audio = Resource::from_str("test.ogg").unwrap();
		assert_eq!(types.filter(&[&img]), vec![false]);
		assert_eq!(types.filter(&[&audio]), vec![true]);
	}
	#[test]
	fn test_match() {
		let types = Mime::try_from(vec!["image/*", "audio/*"]).unwrap();
		let img = Resource::from_str("test.jpg").unwrap();
		let audio = Resource::from_str("test.ogg").unwrap();
		assert_eq!(types.filter(&[&img]), vec![true]);
		assert_eq!(types.filter(&[&audio]), vec![true]);
	}
}
