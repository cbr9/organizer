mod de;

use crate::{config::options::apply::Apply, utils::DefaultOpt};
use serde::Serialize;
use std::str::FromStr;

#[derive(Debug, Serialize, Clone, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ApplyWrapper {
	pub actions: Option<Apply>,
	pub filters: Option<Apply>,
}

impl DefaultOpt for ApplyWrapper {
	fn default_none() -> Self {
		Self {
			actions: None,
			filters: None,
		}
	}

	fn default_some() -> Self {
		Self {
			actions: Some(Apply::default()),
			filters: Some(Apply::default()),
		}
	}
}

impl From<Apply> for ApplyWrapper {
	fn from(val: Apply) -> Self {
		match val {
			Apply::All => Self {
				actions: Some(val.clone()),
				filters: Some(val),
			},
			Apply::Any => Self {
				actions: Some(Apply::All),
				filters: Some(val),
			},
			Apply::AllOf(vec) => Self {
				actions: Some(Apply::AllOf(vec.clone())),
				filters: Some(Apply::AllOf(vec)),
			},
			Apply::AnyOf(vec) => Self {
				actions: Some(Apply::AllOf(vec.clone())),
				filters: Some(Apply::AnyOf(vec)),
			},
		}
	}
}

impl FromStr for ApplyWrapper {
	type Err = serde::de::value::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Self::from(Apply::from_str(s)?))
	}
}
