use anyhow::{Context, Result};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{
	fs::{self, File},
	path::PathBuf,
};

use crate::{
	config::variables::Variable,
	path::prepare::prepare_target_path,
	resource::Resource,
	templates::{template::Template, TemplateEngine},
};

use super::{common::ConflictOption, Action};
use crate::config::actions::common::enabled;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Extract {
	pub to: Template,
	#[serde(default)]
	pub if_exists: ConflictOption,
	#[serde(default = "enabled")]
	enabled: bool,
}

#[typetag::serde(name = "extract")]
impl Action for Extract {
	fn templates(&self) -> Vec<Template> {
		vec![self.to.clone()]
	}

	#[tracing::instrument(ret(level = "info"), err(Debug), level = "debug", skip(template_engine, variables))]
	fn execute(&self, res: &Resource, template_engine: &TemplateEngine, variables: &[Box<dyn Variable>], dry_run: bool) -> Result<Option<PathBuf>> {
		match prepare_target_path(&self.if_exists, res, &self.to, false, template_engine, variables)? {
			Some(dest) => {
				if !dry_run && self.enabled {
					if let Some(parent) = dest.parent() {
						std::fs::create_dir_all(parent).with_context(|| format!("Could not create parent directory for {}", dest.display()))?;
					}
					let file = File::open(&res.path)?;
					let mut archive = zip::ZipArchive::new(file)?;
					archive.extract(&dest)?;

					let content = fs::read_dir(&dest)?.flatten().collect_vec();
					if content.len() == 1 {
						if let Some(dir) = content.first() {
							let dir = dir.path();
							if dir.is_dir() {
								let inner_content = fs::read_dir(&dir)?.flatten().collect_vec();
								let components = dir.components().collect_vec();
								for entry in inner_content {
									let mut new_path: PathBuf = entry.path().components().filter(|c| !components.contains(c)).collect();
									new_path = dest.join(new_path);
									std::fs::rename(entry.path(), new_path)?;
								}
								std::fs::remove_dir(dir)?;
							}
						}
					}
				}
				Ok(Some(dest))
			}
			None => Ok(None),
		}
	}
}
