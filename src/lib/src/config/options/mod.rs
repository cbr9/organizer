use std::{ops::Add, path::PathBuf};

use serde::{Deserialize, Serialize};
mod apply;
pub use apply::*;

#[derive(Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
pub struct Options {
	/// defines whether or not subdirectories must be scanned
	pub recursive: Option<bool>,
	pub watch: Option<bool>,
	pub ignore: Option<Vec<PathBuf>>,
	pub hidden_files: Option<bool>,
	pub apply: Option<ApplyWrapper>,
}

pub trait AsOption<T> {
	fn combine(self, rhs: Self) -> Self
	where
		Self: Sized;
}

impl AsOption<bool> for Option<bool> {
	fn combine(self, rhs: Self) -> Self
	where
		Self: Sized,
	{
		match (&self, &rhs) {
			(None, Some(_)) => rhs,
			(Some(_), None) => self,
			(None, None) => None,
			(Some(_), Some(_)) => rhs,
		}
	}
}

impl AsOption<ApplyWrapper> for Option<ApplyWrapper> {
	fn combine(self, rhs: Self) -> Self
	where
		Self: Sized,
	{
		match (self, rhs) {
			(None, Some(rhs)) => Some(rhs),
			(Some(lhs), None) => Some(lhs),
			(None, None) => None,
			(Some(lhs), Some(rhs)) => Some(ApplyWrapper {
				actions: match (&lhs.actions, &rhs.actions) {
					(None, Some(_)) => rhs.actions,
					(Some(_), None) => lhs.actions,
					(None, None) => None,
					(Some(_), Some(_)) => rhs.actions,
				},
				filters: match (&lhs.filters, &rhs.filters) {
					(None, Some(_)) => rhs.filters,
					(Some(_), None) => lhs.filters,
					(None, None) => None,
					(Some(_), Some(_)) => rhs.filters,
				},
			}),
		}
	}
}

impl<T: Clone> AsOption<Vec<T>> for Option<Vec<T>> {
	fn combine(self, rhs: Self) -> Self
	where
		Self: Sized,
	{
		match (self, rhs) {
			(None, Some(rhs)) => Some(rhs),
			(Some(lhs), None) => Some(lhs),
			(None, None) => None,
			(Some(mut lhs), Some(rhs)) => {
				let mut rhs = rhs;
				let lhs = &mut lhs;
				rhs.append(lhs);
				Some(rhs)
			}
		}
	}
}

impl Add<Self> for &Options {
	type Output = Options;

	fn add(self, rhs: &Options) -> Self::Output {
		Options {
			watch: self.watch.combine(rhs.watch),
			recursive: self.recursive.combine(rhs.recursive),
			hidden_files: self.hidden_files.combine(rhs.hidden_files),
			apply: self.apply.clone().combine(rhs.apply.clone()),
			ignore: self.ignore.clone().combine(rhs.ignore.clone()),
		}
	}
}

#[cfg(test)]
mod tests {
	use std::io::Result;

	use super::*;
	use crate::{settings::Settings, utils::tests::IntoResult};

	#[test]
	fn add_two() -> Result<()> {
		let defaults = Settings::default();
		let opt1 = Options {
			recursive: Some(true),
			watch: None,
			ignore: Some(vec!["$HOME".into(), "$HOME/Downloads".into()]),
			hidden_files: None,
			apply: Some(ApplyWrapper::from(Apply::All)),
		};
		let opt2 = Options {
			recursive: Some(false),
			watch: Some(false),
			ignore: Some(vec!["$HOME/Documents".into()]),
			hidden_files: None,
			apply: Some(ApplyWrapper::from(Apply::Any)),
		};
		let expected = Options {
			recursive: opt2.recursive,
			watch: opt2.watch,
			ignore: Some({
				let mut ignore1 = opt1.clone().ignore.unwrap();
				let ignore2 = &mut opt2.clone().ignore.unwrap();
				ignore2.append(&mut ignore1);
				ignore2.clone()
			}),
			hidden_files: defaults.defaults.hidden_files,
			apply: opt2.apply.clone(),
		};
		(&opt1 + &opt2 == expected).into_result()
	}
	#[test]
	fn add_three() -> Result<()> {
		let opt1 = Options {
			recursive: Some(true),
			watch: None,
			ignore: Some(vec!["$HOME".into(), "$HOME/Downloads".into()]),
			hidden_files: None,
			apply: None,
		};
		let opt2 = Options {
			recursive: Some(false),
			watch: Some(false),
			ignore: Some(vec!["$HOME/Documents".into()]),
			hidden_files: None,
			apply: None,
		};
		let opt3 = Options {
			recursive: Some(true),
			watch: Some(true),
			ignore: Some(vec!["$HOME/Pictures".into()]),
			hidden_files: Some(true),
			apply: Some(ApplyWrapper::from(Apply::Select(vec![0, 2]))),
		};
		let expected = Options {
			recursive: Some(true),
			watch: Some(true),
			ignore: Some({
				let mut ignore1 = opt1.clone().ignore.unwrap();
				let ignore2 = &mut opt2.clone().ignore.unwrap();
				let mut ignore3 = opt3.clone().ignore.unwrap();
				ignore2.append(&mut ignore1);
				ignore3.append(ignore2);
				ignore3
			}),
			hidden_files: Some(true),
			apply: opt3.apply.clone(),
		};
		let one_two = &opt1 + &opt2;
		(&one_two + &opt3 == expected).into_result()
	}
}
