use std::path::PathBuf;

use anyhow::{Context as ErrorContext, Result};
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

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Hardlink {
	to: Template,
	#[serde(default)]
	if_exists: ConflictOption,
	#[serde(default)]
	continue_with: ContinueWith,
	#[serde(default = "enabled")]
	enabled: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
enum ContinueWith {
	Original,
	Link,
}

impl Default for ContinueWith {
	fn default() -> Self {
		Self::Original
	}
}

#[typetag::serde(name = "hardlink")]
impl Action for Hardlink {
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
					std::fs::hard_link(&res.path, &dest)
						.with_context(|| format!("could not create hardlink ({} -> {})", res.path.display(), dest.display()))?;
				}
				if self.continue_with == ContinueWith::Link && self.enabled {
					Ok(Some(dest))
				} else {
					Ok(Some(res.path.clone()))
				}
			}
			None => Ok(None),
		}
	}
}
