use crate::config::AsOption;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum Match {
	All,
	First,
}

impl Default for Match {
	fn default() -> Self {
		Self::First
	}
}

impl AsOption<Match> for Option<Match> {
	fn combine(self, rhs: Self) -> Self {
		match (self, rhs) {
			(None, None) => Some(Match::default()),
			(Some(lhs), None) => Some(lhs),
			(None, Some(rhs)) => Some(rhs),
			(Some(_), Some(rhs)) => Some(rhs),
		}
	}
}
