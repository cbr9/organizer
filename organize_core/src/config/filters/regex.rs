use crate::{config::filters::Filter, resource::Resource, templates::Template};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct Regex(#[serde(deserialize_with = "serde_regex::deserialize", serialize_with = "serde_regex::serialize")] regex::Regex);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct RegularExpression {
	pub pattern: Regex,
	#[serde(default)]
	pub negate: bool,
	pub input: Template,
}

impl PartialEq for Regex {
	fn eq(&self, other: &Self) -> bool {
		self.0.as_str() == other.0.as_str()
	}
}

impl Eq for Regex {}

#[typetag::serde(name = "regex")]
impl Filter for RegularExpression {
	#[tracing::instrument(ret, level = "debug")]
	fn filter(&self, res: &Resource) -> bool {
		self.input.render(&res.context).is_ok_and(|s| {
			let mut matches = self.pattern.0.is_match(&s);
			if self.negate {
				matches = !matches;
			}
			matches
		})
	}
}

#[cfg(test)]
mod tests {}
