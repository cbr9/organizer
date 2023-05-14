use serde::{Deserialize, Serialize};
use strum_macros::Display;

#[derive(Deserialize, Serialize, Debug, Clone, Eq, PartialEq, Display)]
#[strum(serialize_all = "snake_case")]
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
