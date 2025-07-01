use std::fmt;

/// Represents any possible value that can be retrieved from a template variable.
/// This enum provides type safety for the data flowing through the template engine.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
	String(String),
	OptionString(Option<String>),
	// Add other types as needed, e.g., Int(i64), Bool(bool)
	Null,
}

impl fmt::Display for Value {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Value::String(s) => write!(f, "{s}"),
			Value::OptionString(Some(s)) => write!(f, "{s}"),
			Value::OptionString(None) | Value::Null => Ok(()), // Render None/Null as empty string
		}
	}
}
