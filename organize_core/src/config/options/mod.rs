pub mod apply;
mod de;
pub(crate) mod r#match;

use crate::config::options::r#match::Match;

use crate::config::options::apply::wrapper::ApplyWrapper;
use serde::Serialize;
use std::{ops::Add, path::PathBuf};

#[derive(Serialize, Debug, Clone, Eq, PartialEq)]
// #[serde(deny_unknown_fields)]
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
			(None, None) => Some(bool::default()),
			(Some(_), Some(_)) => rhs,
		}
	}
}

impl<T> AsOption<Vec<T>> for Option<Vec<T>>
where
	T: PartialEq + Ord,
{
	fn combine(self, rhs: Self) -> Self {
		match (self, rhs) {
			(None, Some(rhs)) => Some(rhs),
			(Some(lhs), None) => Some(lhs),
			(None, None) => Some(Vec::new()),
			(Some(mut lhs), Some(mut rhs)) => {
				rhs.append(&mut lhs);
				rhs.sort();
				rhs.dedup();
				Some(rhs)
			}
		}
	}
}

impl Add<Self> for Options {
	type Output = Self;

	fn add(self, rhs: Self) -> Self::Output {
		Options {
			recursive: self.recursive.combine(rhs.recursive),
			watch: self.watch.combine(rhs.watch),
			ignore: self.ignore.combine(rhs.ignore),
			hidden_files: self.hidden_files.combine(rhs.hidden_files),
			r#match: self.r#match.combine(rhs.r#match),
			apply: self.apply.combine(rhs.apply),
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::config::options::AsOption;

	#[test]
	fn combine_opt_bool_some_some() {
		let left = Some(true);
		let right = Some(false);
		assert_eq!(left.combine(right), right)
	}
	#[test]
	fn combine_opt_bool_some_none() {
		let left = Some(true);
		let right = None;
		assert_eq!(left.combine(right), left)
	}
	#[test]
	fn combine_opt_bool_none_some() {
		let left = None;
		let right = Some(true);
		assert_eq!(left.combine(right), right)
	}
	#[test]
	fn combine_opt_bool_none_none() {
		let left: Option<bool> = None;
		let right = None;
		assert_eq!(left.combine(right), Some(bool::default()))
	}
	#[test]
	fn combine_opt_vec_none_none() {
		let left: Option<Vec<&str>> = None;
		let right = None;
		assert_eq!(left.combine(right), Some(Vec::new()))
	}
	#[test]
	fn combine_opt_vec_none_some() {
		let left: Option<Vec<&str>> = None;
		let right = Some(vec!["$HOME"]);
		assert_eq!(left.combine(right.clone()), right)
	}
	#[test]
	fn combine_opt_vec_some_none() {
		let left: Option<Vec<&str>> = Some(vec!["$HOME"]);
		let right = None;
		assert_eq!(left.clone().combine(right), left)
	}
	#[test]
	fn combine_opt_vec_some_some_equal() {
		let left = Some(vec!["$HOME", "$HOME/Downloads"]);
		let right = Some(vec!["$HOME", "$HOME/Downloads"]);
		assert_eq!(left.combine(right.clone()), right)
	}
	#[test]
	fn combine_opt_vec_some_some_overlap() {
		let left = Some(vec!["$HOME", "$HOME/Downloads"]);
		let right = Some(vec!["$HOME", "$HOME/Documents"]);
		let expected = Some(vec!["$HOME", "$HOME/Documents", "$HOME/Downloads"]);
		assert_eq!(left.combine(right), expected)
	}
	#[test]
	fn combine_opt_vec_some_some_no_overlap() {
		let left = Some(vec!["$HOME", "$HOME/Downloads"]);
		let right = Some(vec!["$HOME/Documents"]);
		let expected = Some(vec!["$HOME", "$HOME/Documents", "$HOME/Downloads"]);
		assert_eq!(left.combine(right), expected)
	}
}
