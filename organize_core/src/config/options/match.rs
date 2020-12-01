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
