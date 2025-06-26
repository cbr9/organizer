use std::path::{Path, PathBuf};

use anyhow::{Context as ErrorContext, Result};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::{context::RunServices, options::OptionsBuilder, resource::Resource, stdx::path::PathExt};

use super::options::{Options, Target};

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
#[serde(deny_unknown_fields)]
pub struct FolderBuilder {
	pub root: PathBuf,
	#[serde(flatten)]
	pub settings: OptionsBuilder,
}

impl FolderBuilder {
	pub fn build(self, index: usize, defaults: &OptionsBuilder, rule_options: &OptionsBuilder) -> Result<Folder> {
		let options = Options::compile(defaults, rule_options, &self.settings, &self.root);
		Ok(Folder {
			path: self.root,
			settings: options,
			index,
		})
	}
}

/// The final, compiled `Folder` object, ready for execution.
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct Folder {
	pub index: usize,
	pub path: PathBuf,
	pub settings: Options,
}

impl Folder {
	pub async fn get_resources(&self, services: &RunServices) -> Result<Vec<Resource>> {
		let home = &dirs::home_dir().context("unable to find home directory")?;
		let min_depth = {
			let base = if &self.path == home { 1.0 as usize } else { self.settings.min_depth };
			(base as f64).max(1.0) as usize
		};

		let max_depth = if &self.path == home { 1.0 as usize } else { self.settings.max_depth };

		let walker = WalkDir::new(&self.path).min_depth(min_depth).max_depth(max_depth);

		let entries: Vec<Resource> = walker
			.into_iter()
			.filter_entry(|e| self.prefilter(e.path()))
			.flatten()
			.filter(|e| self.postfilter(e.path()))
			.map(|e| Resource::new(e.path()))
			.collect();

		for entry in &entries {
			services.blackboard.resources.insert(entry.to_path_buf(), entry.clone()).await;
		}

		Ok(entries)
	}

	fn prefilter(&self, path: &Path) -> bool {
		if self.settings.exclude.is_empty() {
			return true;
		}
		!self.settings.exclude.iter().any(|dir| {
			if dir.file_name().is_none() || path.file_name().is_none() {
				return false;
			}
			path == dir || path.file_name().unwrap() == dir.file_name().unwrap()
		})
	}

	/// Postfilter applied to each individual entry after it's been discovered.
	fn postfilter(&self, path: &Path) -> bool {
		if path.is_file() && self.settings.target == Target::Folders {
			return false;
		}
		if path.is_dir() && self.settings.target == Target::Files {
			return false;
		}

		if path.is_file() {
			if let Some(extension) = path.extension() {
				let partial_extensions = &["crdownload", "part", "download"];
				if partial_extensions.contains(&&*extension.to_string_lossy()) && !self.settings.partial_files {
					return false;
				}
			}
			if path.is_hidden().unwrap_or(false) && !self.settings.hidden_files {
				return false;
			}
		}
		true
	}
}

pub type Folders = Vec<FolderBuilder>;
