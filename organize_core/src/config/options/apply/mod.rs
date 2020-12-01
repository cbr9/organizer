mod de;
pub mod wrapper;

use crate::config::options::AsOption;
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

impl AsOption<Apply> for Option<Apply> {
	fn combine(&self, rhs: &Self) -> Self {
		match (self, rhs) {
			(None, None) => Some(Apply::default()),
			(Some(lhs), None) => Some(lhs.clone()),
			(None, Some(rhs)) => Some(rhs.clone()),
			(Some(Apply::AllOf(lhs)), Some(Apply::AllOf(rhs))) => {
				let mut lhs = lhs.clone();
				let mut rhs = rhs.clone();
				rhs.append(&mut lhs);
				rhs.sort_unstable();
				rhs.dedup();
				Some(Apply::AllOf(rhs))
			}
			(Some(Apply::AnyOf(lhs)), Some(Apply::AnyOf(rhs))) => {
				let mut lhs = lhs.clone();
				let mut rhs = rhs.clone();
				rhs.append(&mut lhs);
				rhs.sort_unstable();
				rhs.dedup();
				Some(Apply::AnyOf(rhs))
			}
			(_, rhs) => rhs.clone(),
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::config::options::apply::wrapper::ApplyWrapper;

	#[test]
	fn combine_apply_some_some() {
		let left = Some(Apply::All);
		let right = Some(Apply::Any);
		assert_eq!(left.combine(&right), right)
	}

	#[test]
	fn combine_apply_some_none() {
		let left = Some(Apply::All);
		let right = None;
		assert_eq!(left.combine(&right), left)
	}

	#[test]
	fn combine_apply_none_some() {
		let left = None;
		let right = Some(Apply::All);
		assert_eq!(left.combine(&right), right)
	}

	#[test]
	fn combine_apply_vec_some_some_all_of_all_of() {
		let left = Some(Apply::AllOf(vec![0, 1]));
		let right = Some(Apply::AllOf(vec![2]));
		let expected = Some(Apply::AllOf(vec![0, 1, 2]));
		assert_eq!(left.combine(&right), expected)
	}

	#[test]
	fn combine_apply_vec_some_some_any_of_any_of() {
		let left = Some(Apply::AnyOf(vec![0, 1]));
		let right = Some(Apply::AnyOf(vec![2]));
		let expected = Some(Apply::AnyOf(vec![0, 1, 2]));
		assert_eq!(left.combine(&right), expected)
	}

	#[test]
	fn combine_apply_vec_some_some_all_of_any_of() {
		let left = Some(Apply::AllOf(vec![0, 1]));
		let right = Some(Apply::AnyOf(vec![2]));
		assert_eq!(left.combine(&right), right)
	}

	#[test]
	fn combine_apply_vec_some_some_any_of_all_of() {
		let left = Some(Apply::AnyOf(vec![2]));
		let right = Some(Apply::AllOf(vec![0, 1]));
		assert_eq!(left.combine(&right), right)
	}

	#[test]
	fn combine_apply_vec_some_none() {
		let left = Some(Apply::All);
		let right = None;
		assert_eq!(left.combine(&right), left)
	}

	#[test]
	fn combine_apply_vec_none_some() {
		let left = None;
		let right = Some(Apply::All);
		assert_eq!(left.combine(&right), right)
	}

	#[test]
	fn combine_apply_none_none() {
		let left: Option<Apply> = None;
		let right = None;
		assert_eq!(left.combine(&right), Some(Apply::default()))
	}

	#[test]
	fn combine_wrapper_none_none() {
		let left: Option<ApplyWrapper> = None;
		let right = None;
		assert_eq!(left.combine(&right), Some(ApplyWrapper::default()))
	}

	#[test]
	fn combine_wrapper_none_some() {
		let left: Option<ApplyWrapper> = None;
		let right = Some(ApplyWrapper::default());
		assert_eq!(left.combine(&right), right)
	}

	#[test]
	fn combine_wrapper_some_some() {
		let left: Option<ApplyWrapper> = Some(ApplyWrapper {
			actions: Some(Apply::Any),
			filters: None,
		});
		let right = Some(ApplyWrapper {
			actions: None,
			filters: None,
		});
		let expected = Some(ApplyWrapper {
			actions: Some(Apply::Any),
			filters: Some(Apply::All),
		});
		assert_eq!(left.combine(&right), expected)
	}
}
