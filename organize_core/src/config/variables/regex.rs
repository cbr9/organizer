use serde::Deserialize;

use crate::{
	config::filters::regex::RegularExpression,
	templates::{CONTEXT, TERA},
};

use super::AsVariable;

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct RegexVariable {
	pattern: RegularExpression,
	input: String,
}

impl AsVariable for RegexVariable {
	fn register(&self) {
		let mut ctx = CONTEXT.lock().unwrap();
		let input = TERA.lock().unwrap().render_str(&self.input, &ctx).unwrap();
		if let Some(captures) = self.pattern.captures(&input) {
			for name in self.pattern.capture_names().flatten() {
				if let Some(r#match) = captures.name(name) {
					ctx.insert(name, r#match.as_str());
				}
			}
		}
	}
}
