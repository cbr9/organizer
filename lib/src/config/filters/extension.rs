use std::{fmt, ops::Deref, path::Path};

use crate::config::AsFilter;
use serde::{
	de,
	de::{SeqAccess, Visitor},
	export,
	export::PhantomData,
	Deserialize,
	Deserializer,
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Extension(Vec<String>);

impl Deref for Extension {
	type Target = Vec<String>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<'de> Deserialize<'de> for Extension {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct StringOrSeq(PhantomData<fn() -> Extension>);

		impl<'de> Visitor<'de> for StringOrSeq {
			type Value = Extension;

			fn expecting(&self, formatter: &mut export::Formatter) -> fmt::Result {
				formatter.write_str("string or seq")
			}

			fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				Ok(Extension(vec![value.into()]))
			}

			fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
			where
				A: SeqAccess<'de>,
			{
				let mut vec = Vec::new();
				while let Some(val) = seq.next_element()? {
					vec.push(val)
				}
				Ok(Extension(vec))
			}
		}

		deserializer.deserialize_any(StringOrSeq(PhantomData))
	}
}

impl AsFilter for Extension {
	fn matches(&self, path: &Path) -> bool {
		match path.extension() {
			Some(extension) => self.contains(&extension.to_str().unwrap().to_string()),
			None => false,
		}
	}
}

#[cfg(test)]
pub mod tests {
	use serde_test::{assert_de_tokens, Token};
	use std::{
		io::{Error, ErrorKind, Result},
		path::PathBuf,
	};

	use super::Extension;
	use crate::config::AsFilter;

	#[test]
	fn deserialize_string() {
		let value = Extension(vec!["pdf".into()]);
		assert_de_tokens(&value, &[Token::Str("pdf")])
	}
	#[test]
	fn deserialize_seq() {
		let value = Extension(vec!["pdf".into()]);
		assert_de_tokens(&value, &[Token::Seq { len: Some(1) }, Token::Str("pdf"), Token::SeqEnd])
	}
	#[test]
	fn single_match_pdf() {
		let extension = Extension(vec!["pdf".into()]);
		let path = PathBuf::from("$HOME/Downloads/test.pdf");
		assert!(extension.matches(&path))
	}
	#[test]
	fn multiple_match_pdf() {
		let extension = Extension(vec!["pdf".into(), "doc".into(), "docx".into()]);
		let path = PathBuf::from("$HOME/Downloads/test.pdf");
		assert!(extension.matches(&path))
	}
	#[test]
	fn no_match() {
		let extension = Extension(vec!["pdf".into(), "doc".into(), "docx".into()]);
		let path = PathBuf::from("$HOME/Downloads/test.jpg");
		assert!(!extension.matches(&path))
	}
}
