use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

#[derive(Serialize, Default, Debug, Eq, PartialEq, Clone)]
pub struct Template {
	pub text: String,
	pub id: String,
}

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
		val.text
	}
}

impl From<String> for Template {
	fn from(val: String) -> Self {
		Template {
			text: val,
			id: Uuid::new_v4().to_string(),
		}
	}
}

impl From<&str> for Template {
	fn from(val: &str) -> Self {
		Self::from(val.to_string())
	}
}
