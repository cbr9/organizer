use std::path::PathBuf;

use crate::config::{actions::common::enabled, context::ExecutionContext};
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
	fn execute(&self, res: &Resource, ctx: &ExecutionContext) -> Result<Option<PathBuf>> {
		if self.enabled {
			let context = ctx.services.template_engine.context(res);
			if let Some(message) = ctx
				.services
				.template_engine
				.render(&self.message, &context)
				.map_err(anyhow::Error::msg)?
			{
				tracing::info!("{}", message);
			}
		}
		Ok(Some(res.path().to_path_buf()))
	}
}
