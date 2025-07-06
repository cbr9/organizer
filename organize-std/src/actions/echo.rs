use std::sync::Arc;

use async_trait::async_trait;
use organize_sdk::{
	context::ExecutionContext,
	error::Error,
	plugins::action::{Action, ActionBuilder, Receipt},
	templates::template::{Template, TemplateString},
};
use serde::{Deserialize, Serialize};

use anyhow::Result;

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EchoBuilder {
	pub message: TemplateString,
}

#[async_trait]
#[typetag::serde(name = "echo")]
impl ActionBuilder for EchoBuilder {
	async fn build(&self, ctx: &ExecutionContext) -> Result<Box<dyn Action>, Error> {
		let message = ctx.services.template_compiler.compile_template(&self.message)?;
		Ok(Box::new(Echo { message }))
	}
}

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct Echo {
	pub message: Template,
}

#[async_trait]
#[typetag::serde(name = "echo")]
impl Action for Echo {
	async fn commit(&self, ctx: Arc<ExecutionContext>) -> Result<Receipt, Error> {
		self.message
			.render(&ctx)
			.await
			.inspect(|message| tracing::info!("{}", message))?;
		Ok(Receipt {
			next: vec![ctx.scope.resource()?],
			..Default::default()
		})
	}
}
