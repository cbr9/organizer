use crate::{
	config::{context::ExecutionContext, filters::Filter},
	templates::template::Template,
};
use async_trait::async_trait;
use itertools::Itertools;
use serde::{ser::SerializeSeq, Deserialize, Serialize, Serializer};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RegexSet(#[serde(serialize_with = "serialize", deserialize_with = "serde_regex::deserialize")] regex::RegexSet);

impl PartialEq for RegexSet {
	fn eq(&self, other: &Self) -> bool {
		self.patterns().iter().map(|p| p.as_str()).collect_vec() == other.patterns().iter().map(|p| p.as_str()).collect_vec()
	}
}

impl Eq for RegexSet {}

impl std::ops::Deref for RegexSet {
	type Target = regex::RegexSet;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl Default for RegexSet {
	fn default() -> Self {
		Self(regex::RegexSet::empty())
	}
}

pub fn serialize<S: Serializer>(set: &regex::RegexSet, serialize: S) -> Result<S::Ok, S::Error> {
	let p = set.patterns();
	let mut seq = serialize.serialize_seq(Some(p.len()))?;

	for e in p {
		seq.serialize_element(&e)?
	}

	seq.end()
}

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct Regex(#[serde(with = "serde_regex")] regex::Regex);

impl PartialEq for Regex {
	fn eq(&self, other: &Self) -> bool {
		self.as_str() == other.as_str()
	}
}

impl Eq for Regex {}

impl std::ops::Deref for Regex {
	type Target = regex::Regex;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct RegularExpression {
	pub pattern: Regex,
	#[serde(default)]
	pub input: Template,
}

#[async_trait]
#[typetag::serde(name = "regex")]
impl Filter for RegularExpression {
	fn templates(&self) -> Vec<&Template> {
		vec![&self.input]
	}

	async fn filter(&self, ctx: &ExecutionContext) -> bool {
		ctx.services
			.templater
			.render(&self.input, ctx)
			.await
			.unwrap_or_default()
			.is_some_and(|s| self.pattern.is_match(&s))
	}
}

#[cfg(test)]
mod tests {}
