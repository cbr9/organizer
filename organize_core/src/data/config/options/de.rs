use crate::{
	data::config::options::{apply::wrapper::ApplyWrapper, r#match::Match, Options},
	utils::UnwrapOrDefaultOpt,
};
use serde::{
	de::{Error, MapAccess, Visitor},
	Deserialize,
	Deserializer,
};
use std::{fmt, path::PathBuf};

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

			fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
				formatter.write_str("map")
			}

			fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
			where
				A: MapAccess<'de>,
			{
				let mut apply: Option<ApplyWrapper> = None;
				let mut hidden_files: Option<bool> = None;
				let mut ignore: Option<Vec<PathBuf>> = None;
				let mut r#match: Option<Match> = None;
				let mut recursive: Option<bool> = None;
				let mut watch: Option<bool> = None;

				while let Some(key) = map.next_key::<String>()? {
					match key.as_str() {
						"recursive" => {
							recursive = match recursive.is_none() {
								true => Some(map.next_value()?),
								false => return Err(A::Error::duplicate_field("recursive")),
							}
						}
						"watch" => {
							watch = match watch.is_none() {
								true => Some(map.next_value()?),
								false => return Err(A::Error::duplicate_field("watch")),
							}
						}
						"ignore" => {
							ignore = match ignore.is_none() {
								true => Some(map.next_value()?),
								false => return Err(A::Error::duplicate_field("ignore")),
							}
						}
						"hidden_files" => {
							hidden_files = match hidden_files.is_none() {
								true => Some(map.next_value()?),
								false => return Err(A::Error::duplicate_field("hidden_files")),
							}
						}
						"match" => {
							r#match = match r#match.is_none() {
								true => Some(map.next_value()?),
								false => return Err(A::Error::duplicate_field("match")),
							}
						}
						"apply" => {
							apply = match apply.is_none() {
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

				Ok(Options {
					recursive,
					watch,
					ignore,
					hidden_files,
					r#match,
					apply: apply.unwrap_or_default_none(),
				})
			}
		}
		deserializer.deserialize_map(OptVisitor)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde::de::{value::Error, Error as _};
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
}
