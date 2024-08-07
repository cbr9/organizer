use itertools::Itertools;
use serde::Deserialize;
use std::{
	fs::{self, File},
	path::{Path, PathBuf},
};

use crate::{path::prepare_target_path, resource::Resource};

use super::{common::ConflictOption, ActionType, AsAction};

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Extract {
	pub to: PathBuf,
	#[serde(default)]
	pub if_exists: ConflictOption,
	#[serde(default)]
	pub confirm: bool,
}

impl AsAction for Extract {
	const REQUIRES_DEST: bool = true;
	const TYPE: super::ActionType = ActionType::Extract;

	fn get_target_path(&self, src: &Resource) -> anyhow::Result<Option<PathBuf>> {
		let file = File::open(&src.path)?;
		let archive = zip::ZipArchive::new(file)?;
		let mut common_prefix = None;

		for file in archive.file_names() {
			let file_path = Path::new(file);

			// Extract the directory component of the file path
			if let Some(parent) = file_path.parent() {
				if common_prefix.is_none() {
					common_prefix = Some(parent);
				}
			}
		}

		let common_prefix = match common_prefix {
			Some(p) => p.to_path_buf(),
			None => src.path.with_extension(""),
		};

		prepare_target_path(&self.if_exists, src, &self.to.join(common_prefix), false)
	}

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
