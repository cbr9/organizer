use std::{ops::Add, path::PathBuf};

use serde::{Deserialize, Deserializer, Serialize};
mod apply;
mod r#match;
pub use apply::*;
pub use r#match::*;
use serde::de::{Error, MapAccess, Visitor};
use std::{fmt, fmt::Formatter};

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

impl<'de> Deserialize<'de> for Options {
	// for some reason, the derived implementation of Deserialize for Options doesn't return an error
	// when it encounters a key without a value. Instead, it returns None and continues execution.
	// the (hopefully temporary) solution is to implement the deserializer manually
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct OptVisitor;
		impl<'de> Visitor<'de> for OptVisitor {
			type Value = Options;

			fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
				formatter.write_str("map")
			}

			fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
			where
				A: MapAccess<'de>,
			{
				let mut opts = Options {
					recursive: None,
					watch: None,
					ignore: None,
					hidden_files: None,
					r#match: None,
					apply: None,
				};
				while let Some(key) = map.next_key::<String>()? {
					match key.as_str() {
						"recursive" => {
							opts.recursive = match opts.recursive.is_none() {
								true => Some(map.next_value()?),
								false => return Err(A::Error::duplicate_field("recursive")),
							}
						}
						"watch" => {
							opts.watch = match opts.watch.is_none() {
								true => Some(map.next_value()?),
								false => return Err(A::Error::duplicate_field("watch")),
							}
						}
						"ignore" => {
							opts.ignore = match opts.ignore.is_none() {
								true => Some(map.next_value()?),
								false => return Err(A::Error::duplicate_field("ignore")),
							}
						}
						"hidden_files" => {
							opts.hidden_files = match opts.hidden_files.is_none() {
								true => Some(map.next_value()?),
								false => return Err(A::Error::duplicate_field("hidden_files")),
							}
						}
						"match" => {
							opts.r#match = match opts.r#match.is_none() {
								true => Some(map.next_value()?),
								false => return Err(A::Error::duplicate_field("match")),
							}
						}
						"apply" => {
							opts.apply = match opts.apply.is_none() {
								true => Some(map.next_value()?),
								false => return Err(A::Error::duplicate_field("apply")),
							}
						}
						key => {
							return Err(A::Error::unknown_field(key, &[
								"recursive",
								"watch",
								"ignore",
								"hidden_files",
								"match",
								"apply",
							]))
						}
					}
				}
				Ok(opts)
			}
		}
		deserializer.deserialize_map(OptVisitor)
	}
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
	use super::*;
	use serde::de::{value::Error, Error as SerdeError};
	use serde_test::{assert_de_tokens_error, Token};

	fn check_duplicate(field: &'static str, mut token: Vec<Token>) {
		let mut tokens = vec![Token::Map { len: Some(2) }, Token::Str(field)];
		tokens.append(&mut token);
		tokens.push(Token::Str(field));
		tokens.push(Token::MapEnd);

		assert_de_tokens_error::<Options>(tokens.as_slice(), &Error::duplicate_field(field).to_string())
	}

	#[test]
	fn deserialize_duplicates() {
		check_duplicate("recursive", vec![Token::Bool(true)]);
		check_duplicate("watch", vec![Token::Bool(true)]);
		check_duplicate("ignore", vec![Token::Seq { len: Some(1) }, Token::Str("/home"), Token::SeqEnd]);
		check_duplicate("hidden_files", vec![Token::Bool(true)]);
		check_duplicate("match", vec![Token::UnitVariant {
			name: "Match",
			variant: "first",
		}]);
		check_duplicate("apply", vec![Token::UnitVariant {
			name: "Apply",
			variant: "all",
		}]);
	}

	#[test]
	fn deserialize_unknown_field() {
		assert_de_tokens_error::<Options>(
			&[
				Token::Map { len: Some(2) },
				Token::Str("recursive"),
				Token::Bool(true),
				Token::Str("unknown"),
				Token::MapEnd,
			],
			&Error::unknown_field("unknown", &["recursive", "watch", "ignore", "hidden_files", "match", "apply"]).to_string(),
		)
	}

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
