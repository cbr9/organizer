use crate::data::{options::Options, settings::Settings};
use serde::{Deserialize, Deserializer};

impl<'de> Deserialize<'de> for Settings {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		Ok(Self::from(Options::deserialize(deserializer)?))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::utils::DefaultOpt;
	use serde_test::{assert_de_tokens, Token};
	use crate::data::options::recursive::Recursive;

	#[test]
	fn deserialize() {
		let mut defaults = Options::default_none();
		defaults.watch = Some(false);
		defaults.hidden_files = Some(true);
		defaults.recursive = Recursive {
			enabled: Some(true),
			depth: None,
		};
		let value = Settings { defaults };
		assert_de_tokens(
			&value,
			&[
				Token::Map { len: Some(3) },
				Token::Str("hidden_files"),
				Token::Bool(true),
				Token::Str("watch"),
				Token::Bool(false),
				Token::Str("recursive"),
				Token::Bool(true),
				Token::MapEnd,
			],
		)
	}
}
