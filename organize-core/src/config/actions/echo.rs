use crate::{
	config::{
		actions::{common::enabled, Receipt},
		context::ExecutionContext,
	},
	errors::Error,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::templates::template::Template;
use anyhow::Result;

use super::Action;

#[derive(Debug, Clone, Deserialize, Serialize, Default, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Echo {
	message: Template,
	#[serde(default = "enabled")]
	pub enabled: bool,
}

#[async_trait]
#[typetag::serde(name = "echo")]
impl Action for Echo {
	fn templates(&self) -> Vec<&Template> {
		vec![&self.message]
	}

	async fn commit(&self, ctx: &ExecutionContext<'_>) -> Result<Receipt, Error> {
		if self.enabled {
			ctx.services
				.templater
				.render(&self.message, ctx)
				.await?
				.inspect(|message| tracing::info!("{}", message));
		}
		Ok(Receipt {
			next: vec![ctx.scope.resource.clone()],
			..Default::default()
		})
	}
}
