use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::actions::common::enabled;
use crate::{
	config::variables::Variable,
	path::prepare::prepare_target_path,
	resource::Resource,
	templates::{template::Template, TemplateEngine},
};

use super::ActionConfig;
use super::{common::ConflictOption, Action};

#[derive(Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Copy {
	to: Template,
	#[serde(default)]
	if_exists: ConflictOption,
	#[serde(default)]
	continue_with: ContinueWith,
	#[serde(default = "enabled")]
	pub enabled: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
enum ContinueWith {
	Original,
	Copy,
}

impl Default for ContinueWith {
	fn default() -> Self {
		Self::Copy
	}
}

#[typetag::serde(name = "copy")]
impl Action for Copy {
	fn templates(&self) -> Vec<Template> {
		vec![self.to.clone()]
	}

	fn config(&self) -> ActionConfig {
		ActionConfig { parallelize: true }
	}

	#[tracing::instrument(ret(level = "info"), err(Debug), level = "debug", skip(template_engine, variables))]
	fn execute(&self, res: &Resource, template_engine: &TemplateEngine, variables: &[Box<dyn Variable>], dry_run: bool) -> Result<Option<PathBuf>> {
		match prepare_target_path(&self.if_exists, res, &self.to, true, template_engine, variables)? {
			Some(dest) => {
				if !dry_run && self.enabled {
					if let Some(parent) = dest.parent() {
						std::fs::create_dir_all(parent).with_context(|| format!("Could not create parent directory for {}", dest.display()))?;
					}
					std::fs::copy(&res.path, &dest).with_context(|| format!("Could not copy {} -> {}", res.path.display(), dest.display()))?;
				}
				if self.continue_with == ContinueWith::Copy {
					Ok(Some(dest))
				} else {
					Ok(Some(res.path.clone()))
				}
			}
			None => Ok(None),
		}
	}
}
