use std::path::PathBuf;

use crate::{
	resource::Resource,
	templates::{template::Template, TemplateEngine},
};
use anyhow::{Context, Result};
use dialoguer::{theme::ColorfulTheme, Confirm};
use serde::{Deserialize, Serialize};

use super::{Action, ExecutionModel};
use crate::config::actions::common::enabled;

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Eq)]
pub struct Delete {
	#[serde(default = "enabled")]
	enabled: bool,
	#[serde(default = "enabled")]
	confirm: bool,
}

#[typetag::serde(name = "delete")]
impl Action for Delete {
	fn execution_model(&self) -> ExecutionModel {
		if self.confirm {
			ExecutionModel::Linear
		} else {
			ExecutionModel::Parallel
		}
	}
	fn templates(&self) -> Vec<&Template> {
		vec![]
	}

	#[tracing::instrument(ret(level = "info"), err(Debug), level = "debug")]
	fn execute(&self, res: &Resource, _: &TemplateEngine, dry_run: bool) -> Result<Option<PathBuf>> {
		if !dry_run && self.enabled {
			if self.confirm {
				let confirmed = Confirm::with_theme(&ColorfulTheme::default())
					.with_prompt(format!("Delete {}?", res.path.display()))
					.default(false)
					.interact()?;

				if !confirmed {
					// If the user does not confirm, we pass the resource through to the next action.
					return Ok(Some(res.path.clone()));
				}
			}

			if res.path.is_file() {
				std::fs::remove_file(&res.path).with_context(|| format!("could not delete {}", &res.path.display()))?;
			}

			if res.path.is_dir() {
				std::fs::remove_dir_all(&res.path).with_context(|| format!("could not delete {}", &res.path.display()))?;
			}
		}
		Ok(None)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use tempfile;

	#[test]
	fn test_delete() {
		let tmp_dir = tempfile::tempdir().expect("Couldn't create temporary directory");
		let tmp_path = tmp_dir.path().to_owned();
		let tmp_file = tmp_path.join("delete_me.txt");
		let resource = Resource::new(&tmp_file, tmp_dir.path());
		let action = Delete {
			enabled: true,
			confirm: false,
		};

		let template_engine = TemplateEngine::default();

		std::fs::write(&tmp_file, "").expect("Could not create target file");
		assert!(tmp_file.exists());

		action
			.execute(&resource, &template_engine, false)
			.expect("Could not delete target file");
		assert!(!tmp_file.exists());
	}
}
