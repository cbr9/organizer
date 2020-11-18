mod de;

use crate::config::options::{apply::Apply, AsOption};
use serde::Serialize;
use std::str::FromStr;

#[derive(Debug, Serialize, Clone, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ApplyWrapper {
	pub actions: Option<Apply>,
	pub filters: Option<Apply>,
}

impl Default for ApplyWrapper {
	fn default() -> Self {
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

impl AsOption<ApplyWrapper> for Option<ApplyWrapper> {
	fn combine(&self, rhs: &Self) -> Self
	where
		Self: Sized,
	{
		match (self, rhs) {
			(None, Some(rhs)) => Some(rhs.clone()),
			(Some(lhs), None) => Some(lhs.clone()),
			(None, None) => Some(ApplyWrapper::default()),
			(Some(lhs), Some(rhs)) => {
				let wrapper = ApplyWrapper {
					actions: lhs.actions.combine(&rhs.actions),
					filters: lhs.filters.combine(&rhs.filters),
				};
				Some(wrapper)
			}
		}
	}
}
