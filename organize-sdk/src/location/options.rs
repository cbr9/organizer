use serde::{Deserialize, Serialize};
use std::{fmt::Debug, path::PathBuf};

use crate::{context::ExecutionContext, error::Error, templates::template::TemplateString};

fn default_usize() -> usize {
	1.0 as usize
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct OptionsBuilder {
	#[serde(default = "default_usize")]
	pub max_depth: usize,
	#[serde(default = "default_usize")]
	pub min_depth: usize,
	#[serde(default)]
	pub exclude: Vec<TemplateString>,
	#[serde(default)]
	pub hidden_files: bool,
	#[serde(default)]
	pub partial_files: bool,
	#[serde(default)]
	pub follow_symlinks: bool,
	#[serde(default)]
	pub target: Target,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct Options {
	pub max_depth: usize,
	pub min_depth: usize,
	pub exclude: Vec<PathBuf>,
	pub hidden_files: bool,
	pub partial_files: bool,
	pub follow_symlinks: bool,
	pub target: Target,
}

impl OptionsBuilder {
	pub async fn compile(self, ctx: &ExecutionContext<'_>) -> Result<Options, Error> {
		let mut excluded_paths = Vec::new();

		for template in &self.exclude {
			let template = ctx.services.compiler.compile_template(template)?;
			if let Ok(rendered_path_str) = template.render(ctx).await {
				excluded_paths.push(PathBuf::from(rendered_path_str));
			}
		}

		Ok(Options {
			max_depth: self.max_depth,
			min_depth: self.min_depth,
			exclude: excluded_paths,
			hidden_files: self.hidden_files,
			partial_files: self.partial_files,
			follow_symlinks: self.follow_symlinks,
			target: self.target,
		})
	}
}

#[derive(Debug, Default, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Target {
	#[default]
	Files,
	Folders,
}

impl Target {
	/// Returns `true` if the target is [`Files`].
	///
	/// [`Files`]: Target::Files
	#[must_use]
	pub fn is_files(&self) -> bool {
		matches!(self, Self::Files)
	}

	/// Returns `true` if the target is [`Folders`].
	///
	/// [`Folders`]: Target::Folders
	#[must_use]
	pub fn is_folders(&self) -> bool {
		matches!(self, Self::Folders)
	}
}
