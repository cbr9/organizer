use serde::{Deserialize, Serialize};

use crate::{context::ExecutionContext, error::Error, templates::accessor::Accessor};

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum TemplatePart {
	Static(String),
	Dynamic(Box<dyn Accessor>),
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug, Clone)]
pub struct TemplateString(pub String);

impl std::ops::Deref for TemplateString {
	type Target = String;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct Template {
	pub text: String,
	#[serde(skip)]
	pub parts: Vec<TemplatePart>,
}

impl Template {
	pub async fn render(&self, ctx: &ExecutionContext<'_>) -> Result<String, Error> {
		let mut output = String::new();
		for part in &self.parts {
			match part {
				TemplatePart::Static(s) => output.push_str(s),
				TemplatePart::Dynamic(accessor) => {
					let value = accessor.get(ctx).await?;
					output.push_str(&value.to_string());
				}
			}
		}
		Ok(output)
	}
}
