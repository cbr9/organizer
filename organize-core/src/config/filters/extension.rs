use std::borrow::Cow;

use crate::{
	config::{context::ExecutionContext, filters::Filter},
	resource::Resource,
	templates::template::Template,
};
use derive_more::Deref;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Deref, Clone, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Extension {
	#[serde(default)]
	pub extensions: Vec<String>,
}

#[typetag::serde(name = "extension")]
impl Filter for Extension {
	fn templates(&self) -> Vec<&Template> {
		vec![]
	}

	#[tracing::instrument(ret, level = "debug")]
	fn filter(&self, res: &Resource, _: &ExecutionContext) -> bool {
		let extension = res.path().extension().unwrap_or_default().to_string_lossy();
		if extension.is_empty() {
			return false;
		}

		if self.extensions.is_empty() {
			return true;
		}

		self.extensions.iter().any(|e| {
			let mut negate = false;
			let mut parsed = Cow::from(e);

			if parsed.starts_with('!') {
				negate = true;
				parsed = Cow::Owned(parsed.to_mut().replacen('!', "", 1));
			}

			let mut matches = parsed == extension;
			if negate {
				matches = !matches
			}
			matches
		})
	}
}

#[cfg(test)]
pub mod tests {

	use super::Extension;
	use crate::{
		config::{context::ContextHarness, filters::Filter},
		resource::Resource,
		templates::TemplateEngine,
	};

	#[test]
	fn empty_list() {
		let extension = Extension { extensions: vec![] };
		let path = Resource::new("$HOME/Downloads/test.pdf", "").unwrap();
		let harness = ContextHarness::new();
		let context = harness.context();
		assert!(extension.filter(&path, &context))
	}
	#[test]
	fn negative_match() {
		let extension = Extension {
			extensions: vec!["!pdf".into()],
		};
		let path = Resource::new("$HOME/Downloads/test.pdf", "").unwrap();
		let harness = ContextHarness::new();
		let context = harness.context();
		assert!(!extension.filter(&path, &context))
	}
	#[test]
	fn single_match_pdf() {
		let extension = Extension {
			extensions: vec!["pdf".into()],
		};
		let path = Resource::new("$HOME/Downloads/test.pdf", "").unwrap();
		let harness = ContextHarness::new();
		let context = harness.context();
		assert!(extension.filter(&path, &context))
	}
	#[test]
	fn multiple_match_pdf() {
		let extension = Extension {
			extensions: vec!["pdf".into(), "doc".into(), "docx".into()],
		};
		let path = Resource::new("$HOME/Downloads/test.pdf", "").unwrap();
		let harness = ContextHarness::new();
		let context = harness.context();
		assert!(extension.filter(&path, &context))
	}
	#[test]
	fn multiple_match_negative() {
		let extension = Extension {
			extensions: vec!["!pdf".into(), "doc".into(), "docx".into()],
		};
		let path = Resource::new("$HOME/Downloads/test.pdf", "").unwrap();
		let harness = ContextHarness::new();
		let context = harness.context();
		assert!(!extension.filter(&path, &context))
	}

	#[test]
	fn no_match() {
		let extension = Extension {
			extensions: vec!["pdf".into(), "doc".into(), "docx".into()],
		};
		let path = Resource::new("$HOME/Downloads/test.jpg", "").unwrap();
		let harness = ContextHarness::new();
		let context = harness.context();
		assert!(!extension.filter(&path, &context))
	}
}
