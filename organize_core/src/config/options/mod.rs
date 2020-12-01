pub mod apply;
mod de;
pub(crate) mod r#match;

use crate::config::options::r#match::Match;

use crate::{config::options::apply::wrapper::ApplyWrapper, utils::DefaultOpt};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize, Debug, Clone, Eq, PartialEq)]
// #[serde(deny_unknown_fields)]
pub struct Options {
	/// defines whether or not subdirectories must be scanned
	pub recursive: Option<bool>,
	pub watch: Option<bool>,
	pub ignore: Option<Vec<PathBuf>>,
	pub hidden_files: Option<bool>,
	pub r#match: Option<Match>,
	pub apply: ApplyWrapper,
}

impl DefaultOpt for Options {
	fn default_none() -> Self {
		Self {
			recursive: None,
			watch: None,
			ignore: None,
			hidden_files: None,
			r#match: None,
			apply: DefaultOpt::default_none(),
		}
	}

	fn default_some() -> Self {
		Self {
			recursive: Some(false),
			watch: Some(true),
			ignore: Some(Vec::new()),
			hidden_files: Some(false),
			apply: DefaultOpt::default_some(),
			r#match: Some(Match::default()),
		}
	}
}
