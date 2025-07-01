use crate::{
	action::{Action, ActionBuilder, Receipt},
	common::enabled,
	context::ExecutionContext,
	errors::Error,
	templates::template::{Template, TemplateString},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use anyhow::Result;

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EchoBuilder {
	pub message: TemplateString,
	#[serde(default = "enabled")]
	pub enabled: bool,
}

#[async_trait]
#[typetag::serde(name = "echo")]
impl ActionBuilder for EchoBuilder {
	async fn build(&self, ctx: &ExecutionContext<'_>) -> Result<Box<dyn Action>, Error> {
		let message = ctx.services.compiler.compile_template(&self.message)?;
		Ok(Box::new(Echo {
			message,
			enabled: self.enabled,
		}))
	}
}

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct Echo {
	pub message: Template,
	pub enabled: bool,
}

#[async_trait]
#[typetag::serde(name = "echo")]
impl Action for Echo {
	async fn commit(&self, ctx: &ExecutionContext<'_>) -> Result<Receipt, Error> {
		if self.enabled {
			self.message
				.render(ctx)
				.await
				.inspect(|message| tracing::info!("{}", message))?;
		}
		Ok(Receipt {
			next: vec![ctx.scope.resource()?],
			..Default::default()
		})
	}
}
