pub mod recursive;

use crate::path::IsHidden;

use crate::utils::DefaultOpt;

use crate::config::options::recursive::Recursive;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::{folders::Folder, Config, Rule};

#[derive(Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
pub struct FolderOptions {
	/// defines whether or not subdirectories must be scanned
	pub recursive: Option<Recursive>,
	pub ignored_dirs: Option<Vec<PathBuf>>,
	pub hidden_files: Option<bool>,
	pub partial_files: Option<bool>,
}

macro_rules! getters {
	($($v:vis fn $name:ident() -> $typ:ty {$field:tt})+) => {
		impl FolderOptions {
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
	pub fn ignored_dirs() -> Vec<PathBuf> {
		ignored_dirs
	}
	pub fn partial_files() -> bool {
		partial_files
	}
	pub fn hidden_files() -> bool {
		hidden_files
	}
	pub fn recursive() -> Recursive {
		recursive
	}
}

impl FolderOptions {
	pub fn allows(config: &Config, rule: &Rule, folder: &Folder, path: &Path) -> bool {
		let mut allowed = true;

		// filter by partial_files option
		let allows_partial_files = Self::partial_files(config, rule, folder);
		if !allows_partial_files {
			if let Some(extension) = path.extension() {
				let partial_extensions = &["crdownload", "part"];
				let extension = extension.to_string_lossy();
				allowed = allowed && !partial_extensions.contains(&&*extension);
			}
		}

		// filter by hidden_files option
		let allows_hidden_files = Self::hidden_files(config, rule, folder);
		allowed = allowed && ((path.is_hidden() && allows_hidden_files) || !path.is_hidden());

		// filter by ignored_dirs option
		let ignored_dirs = Self::ignored_dirs(config, rule, folder);
		let in_ignored_dir = !ignored_dirs
			.iter()
			.any(|dir: &PathBuf| -> bool { path.parent().map(|parent| dir == parent).unwrap_or_default() });
		allowed = allowed && in_ignored_dir;

		allowed
	}
}

impl Default for FolderOptions {
	fn default() -> Self {
		Self::default_some()
	}
}

impl DefaultOpt for FolderOptions {
	fn default_none() -> Self {
		Self {
			recursive: None,
			ignored_dirs: None,
			hidden_files: None,
			partial_files: None,
		}
	}

	fn default_some() -> Self {
		Self {
			recursive: Some(Recursive::default()),
			ignored_dirs: Some(vec![]),
			hidden_files: Some(false),
			partial_files: Some(false),
		}
	}
}
