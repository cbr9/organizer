use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

#[derive(Serialize, Default, Debug, Clone)]
pub struct Template {
	pub id: String,
	pub input: String,
}

impl PartialEq for Template {
	fn eq(&self, other: &Self) -> bool {
		self.input == other.input
	}
}

impl Eq for Template {}

impl<'de> Deserialize<'de> for Template {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		// Expect the input to be a simple string
		let text = String::deserialize(deserializer)?;
		Ok(Template::from(text))
	}
}

impl From<Template> for String {
	fn from(val: Template) -> Self {
		val.input
	}
}

impl<T: AsRef<str>> From<T> for Template {
	fn from(val: T) -> Self {
		Template {
			input: val.as_ref().to_string(),
			id: Uuid::new_v4().to_string(),
		}
	}
}

#[cfg(test)]
mod tests {

	use super::*;
	use serde_test::{assert_de_tokens, Token};

	#[test]
	fn test_ser_de_empty() {
		let string = "{{ root }}";
		let template = Template::from(string);

		assert_de_tokens(&template, &[Token::String(string)]);
	}
}
