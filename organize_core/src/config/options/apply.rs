use serde::{Deserialize, Serialize};

use crate::utils::DefaultOpt;

#[derive(Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum Apply {
	All,
	Any,
}

impl Default for Apply {
	fn default() -> Self {
		Self::All
	}
}
impl DefaultOpt for Apply {
	fn default_none() -> Self {
		Self::default()
	}

	fn default_some() -> Self {
		Self::default()
	}
}

impl AsRef<Self> for Apply {
	fn as_ref(&self) -> &Self {
		self
	}
}

impl ToString for Apply {
	fn to_string(&self) -> String {
		match self {
			Apply::All => "all".into(),
			Apply::Any => "any".into(),
		}
	}
}
