use std::path::PathBuf;

use crate::{
	config::{
		actions::{common::enabled, Output},
		context::ExecutionContext,
	},
	errors::{Error, ErrorContext},
	templates::Context,
};
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

#[typetag::serde(name = "echo")]
impl Action for Echo {
	fn templates(&self) -> Vec<&Template> {
		vec![&self.message]
	}

	fn execute(&self, ctx: &ExecutionContext) -> Result<Output, Error> {
		if self.enabled {
			let context = Context::new(ctx);

			ctx.services
				.templater
				.render(&self.message, &context)
				.map_err(|e| Error::Template {
					source: e,
					template: self.message.clone(),
					context: ErrorContext::from_scope(&ctx.scope),
				})?
				.inspect(|message| tracing::info!("{}", message));
		}
		Ok(Output::Continue)
	}
}
