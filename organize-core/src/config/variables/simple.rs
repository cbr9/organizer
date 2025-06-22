use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::Variable;
use crate::{
	config::context::ExecutionContext,
	templates::{lazy::LazyVariable, template::Template},
};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct SimpleVariable {
	pub name: String,
	pub value: Template,
}

#[typetag::serde(name = "simple")]
impl Variable for SimpleVariable {
	fn templates(&self) -> Vec<&Template> {
		vec![&self.value]
	}

	fn name(&self) -> &str {
		&self.name
	}

	fn compute(&self, ctx: &ExecutionContext) -> Result<tera::Value> {
		let mut sub_context = tera::Context::new();
		sub_context.insert("path", ctx.scope.resource.as_path());
		sub_context.insert("root", ctx.scope.folder.path.as_path());

		let rendered = ctx.services.templater.render(&self.value, &sub_context)?;
		tera::to_value(rendered).map_err(anyhow::Error::from)
	}
}
