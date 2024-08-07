use tera::Context;

use crate::{config::filters::regex::RegularExpression, templates::TERA};

use super::AsVariable;

impl AsVariable for RegularExpression {
	fn register(&self, context: &mut Context) {
		let input = TERA.lock().unwrap().render_str(&self.input, context).unwrap();
		if let Some(captures) = self.pattern.captures(&input) {
			for name in self.pattern.capture_names().flatten() {
				if let Some(r#match) = captures.name(name) {
					context.insert(name, r#match.as_str());
				}
			}
		}
	}
}
