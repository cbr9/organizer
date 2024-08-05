use std::path::{Path, PathBuf};

use anyhow::{Context as ErrorContext, Result};
use serde::Deserialize;

use crate::{config::SIMULATION, path::prepare_target_path, resource::Resource};

use super::{common::ConflictOption, ActionPipeline, ActionType};

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Move {
	pub to: PathBuf,
	#[serde(default)]
	pub if_exists: ConflictOption,
	#[serde(default)]
	pub confirm: bool,
}

impl ActionPipeline for Move {
	const REQUIRES_DEST: bool = true;
	const TYPE: ActionType = ActionType::Move;

	fn get_target_path(&self, src: &mut Resource) -> Result<Option<PathBuf>> {
		prepare_target_path(&self.if_exists, src, self.to.as_path(), true)
	}

	fn execute<T: AsRef<Path>>(&self, src: &mut Resource, dest: Option<T>) -> Result<Option<PathBuf>> {
		let dest = dest.unwrap();
		if !*SIMULATION {
			std::fs::rename(src.path().as_ref(), dest.as_ref())
				.with_context(|| format!("Could not move {} -> {}", src.path().as_ref().display(), dest.as_ref().display()))?;
		}

		Ok(Some(dest.as_ref().to_path_buf()))
	}
}
