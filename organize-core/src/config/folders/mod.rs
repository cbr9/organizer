use std::path::{Path, PathBuf};

use anyhow::{Context as ErrorContext, Result};
use path_clean::PathClean;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::{
	config::options::OptionsBuilder,
	path::{expand::Expand, is_hidden::IsHidden},
	resource::Resource,
	templates::{template::Template, TemplateEngine},
};

use super::options::{Options, Target};

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
#[serde(deny_unknown_fields)]
pub struct FolderBuilder {
	pub root: Template,
	#[serde(flatten)]
	pub options: OptionsBuilder,
}

impl FolderBuilder {
	pub fn build(
		self,
		index: usize,
		defaults: &OptionsBuilder,
		rule_options: &OptionsBuilder,
		template_engine: &mut TemplateEngine,
	) -> Result<Folder> {
		let path = {
			let context = template_engine.empty_context();
			template_engine
				.tera
				.render_str(&self.root.text, &context)
				.with_context(|| "cannot expand folder name")
				.map(PathBuf::from)
				.map(|p| p.expand_user().clean())?
		};
		let options = Options::compile(defaults, rule_options, &self.options);
		Ok(Folder { path, options, index })
	}
}

/// The final, compiled `Folder` object, ready for execution.
#[derive(Debug, PartialEq, Clone)]
pub struct Folder {
	pub index: usize,
	pub path: PathBuf,
	pub options: Options,
}

impl Folder {
	pub fn get_resources(&self) -> Result<Vec<Resource>> {
		let home = &dirs::home_dir().context("unable to find home directory")?;
		let min_depth = {
			let base = if &self.path == home { 1.0 as usize } else { self.options.min_depth };
			(base as f64).max(1.0) as usize
		};

		let max_depth = if &self.path == home { 1.0 as usize } else { self.options.max_depth };

		let walker = WalkDir::new(&self.path).min_depth(min_depth).max_depth(max_depth);

		let entries = walker
			.into_iter()
			.filter_entry(|e| self.prefilter(e.path()))
			.flatten()
			.filter(|e| self.postfilter(e.path()))
			.flat_map(|e| Resource::new(e.path(), &self.path))
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
