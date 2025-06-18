use std::path::PathBuf;

use crate::{
	config::context::ExecutionContext,
	errors::{ActionError, ErrorContext},
	resource::Resource,
	templates::template::Template,
};
use anyhow::Result;
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
	fn execute(&self, res: &Resource, ctx: &ExecutionContext) -> Result<Option<PathBuf>, ActionError> {
		if !ctx.settings.dry_run && self.enabled {
			if self.confirm {
				let prompt = format!("Delete {}?", res.path().display());
				let confirmed = Confirm::with_theme(&ColorfulTheme::default())
					.with_prompt(&prompt)
					.default(false)
					.interact()
					.map_err(|e| ActionError::Interaction {
						source: e,
						prompt: prompt,
						context: ErrorContext::from_scope(&ctx.scope),
					})
					.inspect_err(|e| tracing::error!(error = ?e))
					.unwrap_or(false);

				if !confirmed {
					// If the user does not confirm, we pass the resource through to the next action.
					return Ok(Some(res.path().to_path_buf()));
				}
			}

			if res.path().is_file() {
				std::fs::remove_file(res.path()).map_err(|e| ActionError::Io {
					source: e,
					path: res.path().to_path_buf(),
					target: None,
					context: ErrorContext::from_scope(&ctx.scope),
				})?;
			} else {
				std::fs::remove_dir_all(res.path()).map_err(|e| ActionError::Io {
					source: e,
					path: res.path().to_path_buf(),
					target: None,
					context: ErrorContext::from_scope(&ctx.scope),
				})?;
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

		let mut harness = ContextHarness::new();
		harness.settings.dry_run = false;
		let context = harness.context();

		std::fs::write(&tmp_file, "").expect("Could not create target file");
		assert!(tmp_file.exists());

		action.execute(&resource, &context).expect("Could not delete target file");
		assert!(!tmp_file.exists());
	}
}
