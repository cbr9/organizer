use crate::{
	config::{context::Context, filters::Filter},
	resource::Resource,
	templates::template::Template,
};
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
	fn templates(&self) -> Vec<&Template> {
		vec![&self.input]
	}

	#[tracing::instrument(ret, level = "debug", skip(ctx))]
	fn filter(&self, res: &Resource, ctx: &Context) -> bool {
		let context = ctx.template_engine.new_context(res);
		ctx.template_engine
			.render(&self.input, &context)
			.unwrap_or_default()
			.is_some_and(|s| {
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
