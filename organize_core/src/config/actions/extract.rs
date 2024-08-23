use itertools::Itertools;
use serde::Deserialize;
use std::{
	fmt::Debug,
	fs::{self, File},
	path::{Path, PathBuf},
};

use crate::{path::prepare::prepare_target_path, resource::Resource, templates::Template};

use super::{common::ConflictOption, script::ActionConfig, AsAction};

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Extract {
	pub to: Template,
	#[serde(default)]
	pub if_exists: ConflictOption,
}

impl AsAction for Extract {
	const CONFIG: ActionConfig = ActionConfig {
		requires_dest: true,
		parallelize: true,
	};

	fn get_target_path(&self, src: &Resource) -> anyhow::Result<Option<PathBuf>> {
		prepare_target_path(&self.if_exists, src, &self.to, false)
	}

	#[tracing::instrument(ret(level = "info"), err(Debug), level = "debug", skip(dest))]
	fn execute<T: AsRef<Path>>(&self, src: &Resource, dest: Option<T>, dry_run: bool) -> anyhow::Result<Option<std::path::PathBuf>> {
		let dest = dest.unwrap().as_ref().to_path_buf();
		if !dry_run {
			let file = File::open(&src.path)?;
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
}
