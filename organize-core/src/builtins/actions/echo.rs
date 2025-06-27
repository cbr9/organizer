use crate::{
	action::{Action, Receipt},
	common::enabled,
	context::ExecutionContext,
	errors::Error,
	templates::template::Template,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use anyhow::Result;

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Echo {
	pub message: Template,
	#[serde(default = "enabled")]
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
