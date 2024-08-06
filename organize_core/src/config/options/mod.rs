pub mod max_depth;

use crate::path::IsHidden;

use crate::utils::DefaultOpt;

use crate::config::options::max_depth::MaxDepth;
use serde::Deserialize;
use std::path::{Path, PathBuf};

use super::{folders::Folder, Config, Rule};

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Options {
	pub max_depth: Option<MaxDepth>,
	pub exclude: Option<Vec<PathBuf>>,
	pub hidden_files: Option<bool>,
	pub partial_files: Option<bool>,
	pub targets: Option<Targets>,
}

#[derive(Debug, Default, Clone, Deserialize, PartialEq)]
pub enum Targets {
	#[default]
	File,
	Dir,
}

macro_rules! getters {
	($($v:vis fn $name:ident() -> $typ:ty {$field:tt})+) => {
		impl Options {
			$($v fn $name(config: &Config, rule: &Rule, folder: &Folder) -> $typ {
				folder.options.$field.as_ref()
					.or(rule.options.$field.as_ref())
					.or(config.defaults.$field.as_ref())
					.or(Self::default_some().$field.as_ref())
					.unwrap()
					.clone()
			})+
		}
	};
}

getters! {
	pub fn excluded() -> Vec<PathBuf> {
		exclude
	}
	pub fn partial_files() -> bool {
		partial_files
	}
	pub fn hidden_files() -> bool {
		hidden_files
	}
	pub fn max_depth() -> MaxDepth {
		max_depth
	}
	pub fn targets() -> Targets {
		targets
	}
}

impl Options {
	pub fn allows_entry<T: AsRef<Path>>(config: &Config, rule: &Rule, folder: &Folder, path: T) -> bool {
		let path = path.as_ref();

		if path.is_file() && Self::targets(config, rule, folder) == Targets::Dir {
			return false;
		}
		if path.is_dir() && Self::targets(config, rule, folder) == Targets::File {
			return false;
		}
		// filter by partial_files option
		if path.is_file() {
			let allows_partial_files = Self::partial_files(config, rule, folder);
			if let Some(extension) = path.extension() {
				let partial_extensions = &["crdownload", "part", "download"];
				let extension = extension.to_string_lossy();
				let is_partial = partial_extensions.contains(&&*extension);
				if is_partial && !allows_partial_files {
					return false;
				}
			}

			// filter by hidden_files option
			let allows_hidden_files = Self::hidden_files(config, rule, folder);
			let is_hidden = path.is_hidden();
			if is_hidden && !allows_hidden_files {
				return false;
			}
		}

		// filter by ignored_dirs option
		let excluded = Self::excluded(config, rule, folder);
		if !excluded.is_empty() {
			let is_ignored = path.ancestors().any(|anc| {
				excluded.iter().any(|dir| {
					if dir.file_name().is_none() || anc.file_name().is_none() {
						return false;
					}
					anc == dir || anc.file_name().unwrap() == dir.file_name().unwrap()
				})
			});
			if is_ignored {
				return false;
			}
		};

		true
	}
}

impl Default for Options {
	fn default() -> Self {
		Self::default_some()
	}
}

impl DefaultOpt for Options {
	fn default_none() -> Self {
		Self {
			max_depth: None,
			exclude: None,
			hidden_files: None,
			partial_files: None,
			targets: None,
		}
	}

	fn default_some() -> Self {
		Self {
			max_depth: Some(MaxDepth::default()),
			exclude: Some(vec![]),
			hidden_files: Some(false),
			partial_files: Some(false),
			targets: Some(Targets::default()),
		}
	}
}
