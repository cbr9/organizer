use regex::RegexVariable;
use serde::Deserialize;
use simple::SimpleVariable;

pub mod regex;
pub mod simple;

pub trait AsVariable {
	fn register(&self);
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all(deserialize = "lowercase"))]
pub enum Variable {
	Simple(SimpleVariable),
	Regex(RegexVariable),
}

impl AsVariable for Variable {
	fn register(&self) {
		match self {
			Variable::Simple(s) => s.register(),
			Variable::Regex(r) => r.register(),
		}
	}
}
