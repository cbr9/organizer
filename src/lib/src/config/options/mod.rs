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

pub fn combine_options<T>(lhs: Option<T>, rhs: Option<T>) -> Option<T> {
	match (&lhs, &rhs) {
		(None, Some(_)) => rhs,
		(Some(_), None) => lhs,
		(None, None) => None,
		(Some(_), Some(_)) => rhs,
	}
}

pub fn combine_option_vec<T: Clone>(lhs: &Option<Vec<T>>, rhs: &Option<Vec<T>>) -> Option<Vec<T>> {
	match (&lhs, &rhs) {
		(None, Some(rhs)) => Some(rhs.clone()),
		(Some(lhs), None) => Some(lhs.clone()),
		(None, None) => None,
		(Some(lhs), Some(rhs)) => {
			let mut rhs = rhs.clone();
			let lhs = &mut lhs.clone();
			rhs.append(lhs);
			Some(rhs)
		}
	}
}

fn combine_apply(lhs: Option<ApplyWrapper>, rhs: Option<ApplyWrapper>) -> Option<ApplyWrapper> {
	match (&lhs, &rhs) {
		(None, Some(_)) => rhs,
		(Some(_), None) => lhs,
		(None, None) => None,
		(Some(lhs), Some(rhs)) => {
			Some(ApplyWrapper {
				// FIXME: avoid cloning
				actions: match (&lhs.actions, &rhs.actions) {
					(None, Some(_)) => rhs.actions.clone(),
					(Some(_), None) => lhs.actions.clone(),
					(None, None) => None,
					(Some(_), Some(_)) => rhs.actions.clone(),
				},
				filters: match (&lhs.filters, &rhs.filters) {
					(None, Some(_)) => rhs.filters.clone(),
					(Some(_), None) => lhs.filters.clone(),
					(None, None) => None,
					(Some(_), Some(_)) => rhs.filters.clone(),
				},
			})
		}
	}
}

impl Add<Self> for &Options {
	type Output = Options;

	fn add(self, rhs: &Options) -> Self::Output {
		Options {
			watch: combine_options(self.watch, rhs.watch),
			recursive: combine_options(self.recursive, rhs.recursive),
			hidden_files: combine_options(self.hidden_files, rhs.hidden_files),
			apply: combine_apply(self.apply.clone(), rhs.apply.clone()),
			ignore: combine_option_vec(&self.ignore, &rhs.ignore),
		}
	}
}

#[cfg(test)]
mod tests {
	use std::io::Result;

	use super::*;
	use crate::utils::tests::IntoResult;
	use crate::settings::Settings;

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
