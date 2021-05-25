mod de;
pub mod wrapper;

use serde::{de::Error, Serialize};
use std::str::FromStr;

#[derive(Serialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum Apply {
	All,
	Any,
	AllOf(Vec<usize>),
	AnyOf(Vec<usize>),
}

impl Default for Apply {
	fn default() -> Self {
		Self::All
	}
}

impl FromStr for Apply {
	type Err = serde::de::value::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"all" => Ok(Self::All),
			"any" => Ok(Self::Any),
			_ => Err(serde::de::value::Error::unknown_variant(s, &["all", "any"])),
		}
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
			Apply::AllOf(_) => "all_of".into(),
			Apply::AnyOf(_) => "any_of".into(),
		}
	}
}
