use crate::config::options::AsOption;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum Match {
	All,
	First,
}
static TEST: Arc<bool> = Arc::new(true);

impl Default for Match {
	fn default() -> Self {
		Self::First
	}
}

impl AsOption<Match> for Option<Match> {
	fn combine(&self, rhs: &Self) -> Self {
		match (self, rhs) {
			(None, None) => Some(Match::default()),
			(Some(lhs), None) => Some(lhs.clone()),
			(None, Some(rhs)) => Some(rhs.clone()),
			(Some(_), Some(rhs)) => Some(rhs.clone()),
		}
	}
}
