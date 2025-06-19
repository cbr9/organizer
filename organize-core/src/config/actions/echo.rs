use std::path::PathBuf;

use crate::{
	config::{actions::common::enabled, context::ExecutionContext},
	errors::{ActionError, ErrorContext},
};
use serde::{Deserialize, Serialize};

use crate::{resource::Resource, templates::template::Template};
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

	#[tracing::instrument(ret(level = "info"), err(Debug), level = "debug", skip(ctx))]
	fn execute(&self, res: &Resource, ctx: &ExecutionContext) -> Result<Option<PathBuf>, ActionError> {
		if self.enabled {
			let context = ctx
				.services
				.templater
				.context()
				.path(res.path())
				.root(res.root())
				.build(&ctx.services.templater);

			ctx.services
				.templater
				.render(&self.message, &context)
				.map_err(|e| ActionError::Template {
					source: e,
					template: self.message.clone(),
					context: ErrorContext::from_scope(&ctx.scope),
				})?
				.inspect(|message| tracing::info!("{}", message));
		}
		Ok(Some(res.path().to_path_buf()))
	}
}
