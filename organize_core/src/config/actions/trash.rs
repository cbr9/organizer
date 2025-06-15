use std::path::PathBuf;

use crate::config::actions::common::enabled;
use crate::templates::template::Template;
use crate::{config::variables::Variable, resource::Resource, templates::TemplateEngine};
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
	fn templates(&self) -> Vec<Template> {
		vec![]
	}

	#[tracing::instrument(ret(level = "info"), err(Debug), level = "debug")]
	fn execute(&self, res: &Resource, _: &TemplateEngine, _: &[Box<dyn Variable>], dry_run: bool) -> Result<Option<PathBuf>> {
		if !dry_run && self.enabled {
			trash::delete(&res.path)?;
		}
		Ok(None)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use tempfile;

	#[test]
	fn test_trash() {
		let tmp_file = tempfile::NamedTempFile::new().unwrap();
		let path = tmp_file.path();
		let resource = Resource::new(path, path.parent().unwrap());
		let action = Trash { enabled: true };
		let template_engine = TemplateEngine::default();
		let variables = vec![];

		assert!(path.exists());

		action
			.execute(&resource, &template_engine, &variables, false)
			.expect("Could not trash target file");
		assert!(!path.exists());
	}
}
