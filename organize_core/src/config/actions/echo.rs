use std::path::PathBuf;

use crate::config::actions::common::enabled;
use serde::{Deserialize, Serialize};

use crate::{
	config::variables::Variable,
	resource::Resource,
	templates::{template::Template, TemplateEngine},
};
use anyhow::Result;

use super::{Action, ActionConfig};

#[derive(Debug, Clone, Deserialize, Serialize, Default, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Echo {
	message: Template,
	#[serde(default = "enabled")]
	pub enabled: bool,
}

#[typetag::serde(name = "echo")]
impl Action for Echo {
	fn config(&self) -> ActionConfig {
		ActionConfig { parallelize: true }
	}
	fn templates(&self) -> Vec<Template> {
		vec![self.message.clone()]
	}

	#[tracing::instrument(ret(level = "info"), err(Debug), level = "debug", skip(template_engine, variables))]
	fn execute(&self, res: &Resource, template_engine: &TemplateEngine, variables: &[Box<dyn Variable>], _: bool) -> Result<Option<PathBuf>> {
		if self.enabled {
			let context = TemplateEngine::new_context(res, variables);
			let message = template_engine.render(&self.message, &context).map_err(anyhow::Error::msg)?;
			tracing::info!("{}", message);
		}
		Ok(Some(res.path.clone()))
	}
}
