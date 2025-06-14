use crate::{
	config::{filters::Filter, variables::Variable},
	resource::Resource,
	templates::{template::Template, TemplateEngine},
};
use itertools::Itertools;
use mime::FromStrError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{convert::TryFrom, ops::Deref, str::FromStr};

impl FromStr for Mime {
	type Err = FromStrError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Self::try_from(vec![s])
	}
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Mime {
	#[serde(deserialize_with = "deserialize_mimetypes", serialize_with = "serialize_mimetypes")]
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

fn serialize_mimetypes<S>(types: &[MimeInternal], serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	let patterns_str: Vec<String> = types
		.iter()
		.map(|mime_internal| {
			if mime_internal.negate {
				format!("!{}", mime_internal.mime)
			} else {
				mime_internal.mime.to_string()
			}
		})
		.collect();
	patterns_str.serialize(serializer)
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

#[typetag::serde(name = "mime")]
impl Filter for Mime {
	fn templates(&self) -> Vec<Template> {
		vec![]
	}
	#[tracing::instrument(ret, level = "debug")]
	fn filter(&self, res: &Resource, _template_engine: &TemplateEngine, _variables: &[Box<dyn Variable>]) -> bool {
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
		let template_engine = TemplateEngine::default();
		let variables = vec![];
		assert!(!types.filter(&img, &template_engine, &variables));
		assert!(types.filter(&audio, &template_engine, &variables))
	}
	#[test]
	fn test_match_negative_one_mime() {
		let types = Mime::try_from(vec!["!image/*"]).unwrap();
		let img = Resource::from_str("test.jpg").unwrap();
		let audio = Resource::from_str("test.ogg").unwrap();
		let template_engine = TemplateEngine::default();
		let variables = vec![];
		assert!(!types.filter(&img, &template_engine, &variables));
		assert!(types.filter(&audio, &template_engine, &variables))
	}
	#[test]
	fn test_match() {
		let types = Mime::try_from(vec!["image/*", "audio/*"]).unwrap();
		let img = Resource::from_str("test.jpg").unwrap();
		let audio = Resource::from_str("test.ogg").unwrap();
		let template_engine = TemplateEngine::default();
		let variables = vec![];
		assert!(types.filter(&img, &template_engine, &variables));
		assert!(types.filter(&audio, &template_engine, &variables))
	}
}
