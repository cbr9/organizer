use crate::user_config::rules::{actions::Actions, filters::Filters, folder::Folders, options::Options};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
	pub actions: Actions,
	pub filters: Filters,
	pub folders: Folders,
	pub options: Option<Options>,
}

impl AsRef<Self> for Rule {
	fn as_ref(&self) -> &Rule {
		self
	}
}
