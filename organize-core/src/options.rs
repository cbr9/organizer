use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use crate::{templates::prelude::Template, utils::backup::BackupLocation};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OptionsBuilder {
	pub max_depth: Option<usize>,
	pub min_depth: Option<usize>,
	pub exclude: Option<Vec<Template>>,
	pub hidden_files: Option<bool>,
	pub partial_files: Option<bool>,
	pub target: Option<Target>,
	pub backup_location: Option<BackupLocation>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct Options {
	pub max_depth: usize,
	pub min_depth: usize,
	pub exclude: Vec<Template>,
	pub hidden_files: bool,
	pub partial_files: bool,
	pub target: Target,
	pub backup_location: BackupLocation,
}

impl Default for Options {
	fn default() -> Self {
		Self {
			max_depth: 1.0 as usize,
			min_depth: 1.0 as usize,
			exclude: Vec::default(),
			hidden_files: bool::default(),
			partial_files: bool::default(),
			target: Target::default(),
			backup_location: BackupLocation::default(),
		}
	}
}

impl Options {
	pub fn compile(defaults: &OptionsBuilder, rule: &OptionsBuilder, folder: &OptionsBuilder) -> Self {
		let fallback = Self::default();

		Self {
			exclude: folder
				.exclude
				.clone()
				.or_else(|| rule.exclude.clone())
				.or_else(|| defaults.exclude.clone())
				.unwrap_or_default(),
			backup_location: folder
				.backup_location
				.clone()
				.or(rule.backup_location.clone())
				.or(defaults.backup_location.clone())
				.unwrap_or(fallback.backup_location),
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

#[derive(Debug, Default, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Target {
	#[default]
	Files,
	Folders,
}
