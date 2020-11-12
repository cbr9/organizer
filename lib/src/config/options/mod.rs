use std::{ops::Add, path::PathBuf};

use serde::{Deserialize, Serialize};
mod apply;
mod r#match;
pub use apply::*;
pub use r#match::*;

#[derive(Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
pub struct Options {
	/// defines whether or not subdirectories must be scanned
	pub recursive: Option<bool>,
	pub watch: Option<bool>,
	pub ignore: Option<Vec<PathBuf>>,
	pub hidden_files: Option<bool>,
	pub r#match: Option<Match>,
	pub apply: Option<ApplyWrapper>,
}

impl Default for Options {
	fn default() -> Self {
		Self {
			recursive: Some(false),
			watch: Some(true),
			ignore: Some(Vec::new()),
			hidden_files: Some(false),
			apply: Some(ApplyWrapper::default()),
			r#match: Some(Match::default()),
		}
	}
}

pub trait AsOption<T: Default> {
	fn combine(self, rhs: Self) -> Self;
}

impl AsOption<Options> for Option<Options> {
	fn combine(self, rhs: Self) -> Self {
		match (self, rhs) {
			(None, None) => Some(Options::default()),
			(Some(lhs), None) => Some(lhs),
			(None, Some(rhs)) => Some(rhs),
			(Some(lhs), Some(rhs)) => Some(lhs + rhs),
		}
	}
}

impl AsOption<bool> for Option<bool> {
	fn combine(self, rhs: Self) -> Self {
		match (&self, &rhs) {
			(None, Some(_)) => rhs,
			(Some(_), None) => self,
			(None, None) => None,
			(Some(_), Some(_)) => rhs,
		}
	}
}

impl<T> AsOption<Vec<T>> for Option<Vec<T>> {
	fn combine(self, rhs: Self) -> Self {
		match (self, rhs) {
			(None, Some(rhs)) => Some(rhs),
			(Some(lhs), None) => Some(lhs),
			(None, None) => None,
			(Some(mut lhs), Some(mut rhs)) => {
				rhs.append(&mut lhs);
				Some(rhs)
			}
		}
	}
}

impl Add<Self> for Options {
	type Output = Self;

	fn add(self, rhs: Self) -> Self::Output {
		Options {
			watch: self.watch.combine(rhs.watch),
			recursive: self.recursive.combine(rhs.recursive),
			hidden_files: self.hidden_files.combine(rhs.hidden_files),
			apply: self.apply.combine(rhs.apply),
			ignore: self.ignore.combine(rhs.ignore),
			r#match: self.r#match.combine(rhs.r#match),
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
			r#match: Some(Match::First),
		};
		let opt2 = Options {
			recursive: Some(false),
			watch: Some(false),
			ignore: Some(vec!["$HOME/Documents".into()]),
			hidden_files: None,
			apply: Some(ApplyWrapper::from(Apply::Any)),
			r#match: None,
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
			r#match: Some(Match::First),
		};
		(opt1 + opt2 == expected).into_result()
	}
	#[test]
	fn add_three() -> Result<()> {
		let opt1 = Options {
			recursive: Some(true),
			watch: None,
			ignore: Some(vec!["$HOME".into(), "$HOME/Downloads".into()]),
			hidden_files: None,
			apply: None,
			r#match: Some(Match::All),
		};
		let opt2 = Options {
			recursive: Some(false),
			watch: Some(false),
			ignore: Some(vec!["$HOME/Documents".into()]),
			hidden_files: None,
			apply: None,
			r#match: Some(Match::First),
		};
		let opt3 = Options {
			recursive: Some(true),
			watch: Some(true),
			ignore: Some(vec!["$HOME/Pictures".into()]),
			hidden_files: Some(true),
			apply: Some(ApplyWrapper::from(Apply::Select(vec![0, 2]))),
			r#match: None,
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
			r#match: Some(Match::First),
		};
		(opt1 + opt2 + opt3 == expected).into_result()
	}
}
