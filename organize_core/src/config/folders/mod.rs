use std::path::{Path, PathBuf};

use anyhow::{Context as ErrorContext, Result};
use path_clean::PathClean;
use serde::{Deserialize, Serialize};
use tera::Context;
use walkdir::WalkDir;

use crate::{
	config::options::OptionsBuilder,
	path::{expand::Expand, is_hidden::IsHidden},
	resource::Resource,
	templates::Template,
};

use super::{
	options::{Options, Target},
	variables::Variable,
};

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct FolderBuilder {
	root: Template,
	#[serde(flatten)]
	pub options: OptionsBuilder,
}

impl FolderBuilder {
	pub fn build(self, defaults: &OptionsBuilder, rule_options: &OptionsBuilder) -> Result<Folder> {
		let path = {
			let context = Context::new();
			self.root
				.render(&context)
				.with_context(|| "cannot expand folder name")
				.map(PathBuf::from)
				.map(|p| p.expand_user().clean())?
		};
		let options = Options::compile(defaults, rule_options, &self.options);
		Ok(Folder { path, options })
	}
}

/// The final, compiled `Folder` object, ready for execution.
#[derive(Debug, PartialEq, Clone)]
pub struct Folder {
	pub path: PathBuf,
	pub options: Options,
}

impl Folder {
	pub fn get_resources(&self, rule_variables: &[Box<dyn Variable>]) -> Result<Vec<Resource>> {
		let home = &dirs::home_dir().context("unable to find home directory")?;
		let min_depth = if &self.path == home { 1.0 } else { self.options.min_depth };
		let max_depth = if &self.path == home { 1.0 } else { self.options.max_depth };

		let walker = WalkDir::new(&self.path)
			.min_depth(min_depth.max(1.0) as usize)
			.max_depth(max_depth as usize);

		let entries = walker
			.into_iter()
			.filter_entry(|e| self.prefilter(e.path()))
			.flatten()
			.filter(|e| self.postfilter(e.path()))
			.map(|e| Resource::new(e.path(), &self.path, rule_variables.to_vec()))
			.collect();

		Ok(entries)
	}

	fn prefilter(&self, path: &Path) -> bool {
		if self.options.exclude.is_empty() {
			return true;
		}
		!self.options.exclude.iter().any(|dir| {
			if dir.file_name().is_none() || path.file_name().is_none() {
				return false;
			}
			path == dir || path.file_name().unwrap() == dir.file_name().unwrap()
		})
	}

	/// Postfilter applied to each individual entry after it's been discovered.
	fn postfilter(&self, path: &Path) -> bool {
		if path.is_file() && self.options.target == Target::Folders {
			return false;
		}
		if path.is_dir() && self.options.target == Target::Files {
			return false;
		}

		if path.is_file() {
			if let Some(extension) = path.extension() {
				let partial_extensions = &["crdownload", "part", "download"];
				if partial_extensions.contains(&&*extension.to_string_lossy()) && !self.options.partial_files {
					return false;
				}
			}
			if path.is_hidden().unwrap_or(false) && !self.options.hidden_files {
				return false;
			}
		}
		true
	}
}

pub type Folders = Vec<FolderBuilder>;
