use crate::{path::is_hidden::IsHidden, resource::Resource};
use anyhow::{Context, Result};

use crate::utils::DefaultOpt;

use serde::Deserialize;
use std::{
	fmt::Debug,
	path::{Path, PathBuf},
};
use walkdir::WalkDir;

use super::{folders::Folder, Config, Rule};

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Options {
	pub max_depth: Option<f64>,
	pub min_depth: Option<f64>,
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
	pub fn max_depth() -> f64 {
		max_depth
	}
	pub fn min_depth() -> f64 {
		min_depth
	}
	pub fn targets() -> Targets {
		targets
	}
}

impl Options {
	fn walker(config: &Config, rule: &Rule, folder: &Folder) -> Result<WalkDir> {
		let path = &folder.path()?;
		let home = &dirs::home_dir().context("unable to find home directory")?;
		let max_depth = if path == home { 1.0 } else { Self::max_depth(config, rule, folder) };
		let min_depth = if path == home { 1.0 } else { Self::min_depth(config, rule, folder) };
		Ok(WalkDir::new(path)
			.min_depth(min_depth.max(1.0) as usize)
			.max_depth(max_depth as usize))
	}

	pub fn get_entries(config: &Config, rule: &Rule, folder: &Folder) -> Result<Vec<Resource>> {
		let location = folder.path()?;
		Ok(Self::walker(config, rule, folder)?
			.into_iter()
			.filter_entry(|e| Options::prefilter(config, rule, folder, e.path()))
			.flatten()
			.filter(|e| Options::postfilter(config, rule, folder, e.path()))
			.map(|e| Resource::new(e.path(), &location, rule.variables.to_vec()))
			.collect())
	}

	#[tracing::instrument(ret, level = "debug", skip(config, rule, folder))]
	pub fn prefilter<T: AsRef<Path> + Debug>(config: &Config, rule: &Rule, folder: &Folder, path: T) -> bool {
		let mut excluded = Self::excluded(config, rule, folder);
		if excluded.is_empty() {
			return true;
		}

		excluded.sort_by_key(|path| path.components().count());

		!excluded.iter().any(|dir| {
			if dir.file_name().is_none() || path.as_ref().file_name().is_none() {
				return false;
			}
			path.as_ref() == dir || path.as_ref().file_name().unwrap() == dir.file_name().unwrap()
		})
	}

	#[tracing::instrument(ret, level = "debug", skip(config, rule, folder))]
	pub fn postfilter<T: AsRef<Path> + Debug>(config: &Config, rule: &Rule, folder: &Folder, path: T) -> bool {
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
			let is_hidden = path.is_hidden().unwrap();
			if is_hidden && !allows_hidden_files {
				return false;
			}
		}

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
			min_depth: None,
		}
	}

	fn default_some() -> Self {
		Self {
			max_depth: Some(1.0),
			min_depth: Some(1.0),
			exclude: Some(vec![]),
			hidden_files: Some(false),
			partial_files: Some(false),
			targets: Some(Targets::default()),
		}
	}
}
