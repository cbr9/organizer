use crate::data::config::filters::extension::Extension;
use serde::{
	de,
	de::{SeqAccess, Visitor},
	Deserialize, Deserializer,
};
use std::fmt;
use std::marker::PhantomData;

impl<'de> Deserialize<'de> for Extension {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct StringOrSeq(PhantomData<fn() -> Extension>);

		impl<'de> Visitor<'de> for StringOrSeq {
			type Value = Extension;

			fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
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

#[cfg(test)]
mod tests {
	use serde_test::{assert_de_tokens, Token};

	use super::Extension;

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
}
