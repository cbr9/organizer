use std::path::PathBuf;

use crate::config::actions::common::enabled;
use anyhow::{Context as ErrorContext, Result};
use serde::{Deserialize, Serialize};

use crate::{
	config::variables::Variable,
	path::prepare::prepare_target_path,
	resource::Resource,
	templates::{template::Template, TemplateEngine},
};

use super::{common::ConflictOption, Action};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Move {
	pub to: Template,
	#[serde(default)]
	pub if_exists: ConflictOption,
	#[serde(default = "enabled")]
	enabled: bool,
}

#[typetag::serde(name = "move")]
impl Action for Move {
	fn templates(&self) -> Vec<&Template> {
		vec![&self.to]
	}

	#[tracing::instrument(ret(level = "info"), err(Debug), level = "debug", skip(template_engine, variables))]
	fn execute(&self, res: &Resource, template_engine: &TemplateEngine, variables: &[Box<dyn Variable>], dry_run: bool) -> Result<Option<PathBuf>> {
		match prepare_target_path(&self.if_exists, res, &self.to, true, template_engine, variables)? {
			Some(dest) => {
				if !dry_run && self.enabled {
					if let Some(parent) = dest.parent() {
						std::fs::create_dir_all(parent).with_context(|| format!("Could not create parent directory for {}", dest.display()))?;
					}
					std::fs::rename(&res.path, &dest).with_context(|| format!("Could not move {} -> {}", res.path.display(), dest.display()))?;
				}
				Ok(Some(dest))
			}
			None => Ok(None),
		}
	}
}
