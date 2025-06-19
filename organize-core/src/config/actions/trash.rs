use std::path::PathBuf;

use crate::{
	config::{
		actions::{common::enabled, Output},
		context::ExecutionContext,
	},
	errors::{Error, ErrorContext},
	templates::template::Template,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::Action;

#[derive(Debug, Clone, Serialize, Default, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Trash {
	#[serde(default = "enabled")]
	enabled: bool,
}

#[typetag::serde(name = "trash")]
impl Action for Trash {
	fn templates(&self) -> Vec<&Template> {
		vec![]
	}

	fn execute(&self, ctx: &ExecutionContext) -> Result<Output, Error> {
		if !ctx.settings.dry_run && self.enabled {
			trash::delete(ctx.scope.resource.path()).map_err(|e| Error::Trash {
				source: e,
				path: ctx.scope.resource.path().to_path_buf(),
				context: ErrorContext::from_scope(&ctx.scope),
			})?;
		}
		Ok(Output::Stop)
	}
}

// #[cfg(test)]
// mod tests {
// 	use crate::config::context::ContextHarness;

// 	use super::*;
// 	use tempfile;

// 	#[test]
// 	fn test_trash() {
// 		let tmp_file = tempfile::NamedTempFile::new().unwrap();
// 		let path = tmp_file.path();
// 		let resource = Resource::new(path, Some(path.parent().unwrap())).unwrap();
// 		let action = Trash { enabled: true };

// 		assert!(path.exists());
// 		let mut harness = ContextHarness::new();
// 		harness.settings.dry_run = false;
// 		let context = harness.context();

// 		action.execute(&resource, &context).expect("Could not trash target file");
// 		assert!(!path.exists());
// 	}
// }
