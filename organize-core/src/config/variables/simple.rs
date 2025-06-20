
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

	fn compute(&self, ctx: &ExecutionContext) -> Result<tera::Value> {
		let mut sub_context = tera::Context::new();

		for var in &ctx.scope.rule.variables {
			if var.name() == self.name() {
				continue;
			}

			let lazy_value = LazyVariable { variable: var, context: ctx };
			sub_context.insert(var.name(), &lazy_value);
		}

		sub_context.insert("path", ctx.scope.resource.path());
		if let Some(root) = ctx.scope.resource.root() {
			sub_context.insert("root", root);
		}

		let rendered = ctx.services.templater.render(&self.value, &sub_context)?;
		tera::to_value(rendered).map_err(anyhow::Error::from)
	}
}
