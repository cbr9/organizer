use std::path::PathBuf;

use crate::{config::context::ExecutionContext, resource::Resource, templates::template::Template};
use anyhow::{Context as _, Result};
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

	#[tracing::instrument(ret(level = "info"), err(Debug), level = "debug", skip(ctx))]
	fn execute(&self, res: &Resource, ctx: &ExecutionContext) -> Result<Option<PathBuf>> {
		if !ctx.settings.dry_run && self.enabled {
			if self.confirm {
				let confirmed = Confirm::with_theme(&ColorfulTheme::default())
					.with_prompt(format!("Delete {}?", res.path().display()))
					.default(false)
					.interact()?;

				if !confirmed {
					// If the user does not confirm, we pass the resource through to the next action.
					return Ok(Some(res.path().to_path_buf()));
				}
			}

			if res.path().is_file() {
				std::fs::remove_file(res.path()).with_context(|| format!("could not delete {}", &res.path().display()))?;
			}

			if res.path().is_dir() {
				std::fs::remove_dir_all(res.path()).with_context(|| format!("could not delete {}", &res.path().display()))?;
			}
		}
		Ok(None)
	}
}

#[cfg(test)]
mod tests {
	use crate::config::context::ContextHarness;

	use super::*;
	use tempfile;

	#[test]
	fn test_delete() {
		let tmp_dir = tempfile::tempdir().expect("Couldn't create temporary directory");
		let tmp_path = tmp_dir.path().to_owned();
		let tmp_file = tmp_path.join("delete_me.txt");
		let resource = Resource::new(&tmp_file, tmp_dir.path()).unwrap();
		let action = Delete {
			enabled: true,
			confirm: false,
		};

		let harness = ContextHarness::new();
		let context = harness.context();

		std::fs::write(&tmp_file, "").expect("Could not create target file");
		assert!(tmp_file.exists());

		action.execute(&resource, &context).expect("Could not delete target file");
		assert!(!tmp_file.exists());
	}
}
