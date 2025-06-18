use anyhow::Result;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{
	fs::{self, File},
	path::PathBuf,
};
use zip::result::ZipError;

use crate::{
	config::context::ExecutionContext,
	errors::{ActionError, ErrorContext},
	path::prepare::prepare_target_path,
	resource::Resource,
	templates::template::Template,
};

use super::{common::ConflictResolution, Action};
use crate::config::actions::common::enabled;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Extract {
	pub to: Template,
	#[serde(default, rename = "if_exists")]
	pub on_conflict: ConflictResolution,
	#[serde(default = "enabled")]
	enabled: bool,
}

#[typetag::serde(name = "extract")]
impl Action for Extract {
	fn templates(&self) -> Vec<&Template> {
		vec![&self.to]
	}

	#[tracing::instrument(ret(level = "info"), err(Debug), level = "debug", skip(ctx))]
	fn execute(&self, res: &Resource, ctx: &ExecutionContext) -> Result<Option<PathBuf>, ActionError> {
		match prepare_target_path(&self.on_conflict, res, &self.to, false, ctx)? {
			Some(target) => {
				if !ctx.settings.dry_run && self.enabled {
					let map_io = |e: std::io::Error| ActionError::Io {
						source: e,
						path: res.path().to_path_buf(),
						target: Some(target.clone().to_path_buf()),
						context: ErrorContext::from_scope(&ctx.scope),
					};
					let map_zip = |e: ZipError| ActionError::Extraction {
						source: e,
						path: res.path().to_path_buf(),
						context: ErrorContext::from_scope(&ctx.scope),
					};
					let file = File::open(res.path()).map_err(map_io)?;

					let mut archive = zip::ZipArchive::new(file).map_err(map_zip)?;

					archive.extract(&target).map_err(map_zip)?;

					let content = fs::read_dir(&target).map_err(map_io)?.flatten().collect_vec();

					if content.len() == 1 {
						if let Some(dir) = content.first() {
							let dir = dir.path();
							if dir.is_dir() {
								let inner_content = fs::read_dir(&dir).map_err(map_io)?.flatten().collect_vec();
								let components = dir.components().collect_vec();
								for entry in inner_content {
									let mut new_path: PathBuf = entry.path().components().filter(|c| !components.contains(c)).collect();
									new_path = target.join(new_path);
									std::fs::rename(entry.path(), new_path).map_err(map_io)?;
								}
								std::fs::remove_dir(dir).map_err(map_io)?;
							}
						}
					}
				}
				Ok(Some(target.to_path_buf()))
			}
			None => Ok(None),
		}
	}
}
