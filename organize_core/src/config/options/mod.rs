pub mod apply;
pub(crate) mod r#match;
pub mod recursive;

use crate::config::options::r#match::Match;

use crate::utils::DefaultOpt;

use crate::config::options::recursive::Recursive;
use anyhow::{Context, Result};
use apply::Apply;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
pub struct Options {
	/// defines whether or not subdirectories must be scanned
	pub recursive: Recursive,
	pub watch: Option<bool>,
	pub ignored_dirs: Option<Vec<PathBuf>>,
	pub hidden_files: Option<bool>,
	pub r#match: Option<Match>,
	pub partial_files: Option<bool>,
	pub apply: Option<Apply>,
}

impl Options {
	pub fn parse<T: AsRef<Path>>(path: T) -> Result<Self> {
		let path = path.as_ref();
		fs::read_to_string(path).map(|s| toml::from_str(&s).with_context(|| format!("could not deserialize {}", path.display())))?
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
			recursive: DefaultOpt::default_none(),
			watch: None,
			ignored_dirs: None,
			hidden_files: None,
			partial_files: None,
			r#match: None,
			apply: Some(Apply::All),
		}
	}

	fn default_some() -> Self {
		Self {
			recursive: DefaultOpt::default_some(),
			watch: Some(true),
			ignored_dirs: Some(Vec::new()),
			hidden_files: Some(false),
			partial_files: Some(false),
			apply: Some(Default::default()),
			r#match: Some(Match::default()),
		}
	}
}
