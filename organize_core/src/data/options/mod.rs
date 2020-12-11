pub mod apply;
mod de;
pub(crate) mod r#match;

use crate::data::options::r#match::Match;

use crate::{data::options::apply::wrapper::ApplyWrapper, utils::DefaultOpt};

use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize, Debug, Clone, Eq, PartialEq)]
pub struct Options {
	/// defines whether or not subdirectories must be scanned
	pub recursive: Option<bool>,
	pub watch: Option<bool>,
	pub ignored_dirs: Option<Vec<PathBuf>>,
	pub hidden_files: Option<bool>,
	pub r#match: Option<Match>,
	#[serde(default = "DefaultOpt::default_none")]
	pub apply: ApplyWrapper,
}

impl DefaultOpt for Options {
	fn default_none() -> Self {
		Self {
			recursive: None,
			watch: None,
			ignored_dirs: None,
			hidden_files: None,
			r#match: None,
			apply: DefaultOpt::default_none(),
		}
	}

	fn default_some() -> Self {
		Self {
			recursive: Some(false),
			watch: Some(true),
			ignored_dirs: Some(Vec::new()),
			hidden_files: Some(false),
			apply: DefaultOpt::default_some(),
			r#match: Some(Match::default()),
		}
	}
}
