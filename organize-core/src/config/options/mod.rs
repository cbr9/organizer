use serde::{Deserialize, Serialize};
use std::{fmt::Debug, path::PathBuf};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OptionsBuilder {
	pub max_depth: Option<f64>,
	pub min_depth: Option<f64>,
	pub exclude: Option<Vec<PathBuf>>,
	pub hidden_files: Option<bool>,
	pub partial_files: Option<bool>,
	pub target: Option<Target>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Options {
	pub max_depth: f64,
	pub min_depth: f64,
	pub exclude: Vec<PathBuf>,
	pub hidden_files: bool,
	pub partial_files: bool,
	pub target: Target,
}

impl Default for Options {
	fn default() -> Self {
		Self {
			max_depth: 1.0,
			min_depth: 1.0,
			exclude: Vec::new(),
			hidden_files: false,
			partial_files: false,
			target: Target::default(),
		}
	}
}

impl Options {
	pub fn compile(defaults: &OptionsBuilder, rule: &OptionsBuilder, folder: &OptionsBuilder) -> Self {
		// Establish the ultimate fallback defaults for any un-defined option
		let fallback = Self::default();

		Self {
			max_depth: folder
				.max_depth
				.or(rule.max_depth)
				.or(defaults.max_depth)
				.unwrap_or(fallback.max_depth),
			min_depth: folder
				.min_depth
				.or(rule.min_depth)
				.or(defaults.min_depth)
				.unwrap_or(fallback.min_depth),
			exclude: folder
				.exclude
				.clone()
				.or_else(|| rule.exclude.clone())
				.or_else(|| defaults.exclude.clone())
				.unwrap_or(fallback.exclude),
			hidden_files: folder
				.hidden_files
				.or(rule.hidden_files)
				.or(defaults.hidden_files)
				.unwrap_or(fallback.hidden_files),
			partial_files: folder
				.partial_files
				.or(rule.partial_files)
				.or(defaults.partial_files)
				.unwrap_or(fallback.partial_files),
			target: folder
				.target
				.clone()
				.or_else(|| rule.target.clone())
				.or_else(|| defaults.target.clone())
				.unwrap_or(fallback.target),
		}
	}
}

#[derive(Debug, Default, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Target {
	#[default]
	Files,
	Folders,
}
