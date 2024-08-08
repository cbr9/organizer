use regex::RegexVariable;
use serde::Deserialize;
use simple::SimpleVariable;
use tera::Context;

use super::filters::regex::RegularExpression;

pub mod regex;
pub mod simple;

pub trait AsVariable {
	fn register(&self, context: &mut Context);
}

#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all(deserialize = "lowercase"))]
pub enum Variable {
	Simple(SimpleVariable),
	Regex(RegexVariable),
}

impl AsVariable for Variable {
	fn register(&self, context: &mut Context) {
		match self {
			Variable::Simple(s) => s.register(context),
			Variable::Regex(r) => r.register(context),
		}
	}
}
