use std::borrow::Cow;

use crate::{
	config::{context::ExecutionContext, filters::Filter},
	templates::template::Template,
};
use async_trait::async_trait;
use derive_more::Deref;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Deref, Clone, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Extension {
	#[serde(default)]
	pub extensions: Vec<String>,
}

#[async_trait]
#[typetag::serde(name = "extension")]
impl Filter for Extension {
	fn templates(&self) -> Vec<&Template> {
		vec![]
	}

	async fn filter(&self, ctx: &ExecutionContext) -> bool {
		let extension = ctx.scope.resource.path().extension().unwrap_or_default().to_string_lossy();
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

// #[cfg(test)]
// pub mod tests {

// 	use std::path::PathBuf;

// 	use super::Extension;
// 	use crate::{
// 		config::{context::ContextHarness, filters::Filter},
// 		resource::Resource,
// 	};

// 	#[test]
// 	fn empty_list() {
// 		let extension = Extension { extensions: vec![] };
// 		let path = Resource::new::<_, PathBuf>("$HOME/Downloads/test.pdf", None).unwrap();
// 		let harness = ContextHarness::new();
// 		let context = harness.context();
// 		assert!(extension.filter(&path, &context))
// 	}
// 	#[test]
// 	fn negative_match() {
// 		let extension = Extension {
// 			extensions: vec!["!pdf".into()],
// 		};
// 		let path = Resource::new::<_, PathBuf>("$HOME/Downloads/test.pdf", None).unwrap();
// 		let harness = ContextHarness::new();
// 		let context = harness.context();
// 		assert!(!extension.filter(&path, &context))
// 	}
// 	#[test]
// 	fn single_match_pdf() {
// 		let extension = Extension {
// 			extensions: vec!["pdf".into()],
// 		};
// 		let path = Resource::new::<_, PathBuf>("$HOME/Downloads/test.pdf", None).unwrap();
// 		let harness = ContextHarness::new();
// 		let context = harness.context();
// 		assert!(extension.filter(&path, &context))
// 	}
// 	#[test]
// 	fn multiple_match_pdf() {
// 		let extension = Extension {
// 			extensions: vec!["pdf".into(), "doc".into(), "docx".into()],
// 		};
// 		let path = Resource::new::<_, PathBuf>("$HOME/Downloads/test.pdf", None).unwrap();
// 		let harness = ContextHarness::new();
// 		let context = harness.context();
// 		assert!(extension.filter(&path, &context))
// 	}
// 	#[test]
// 	fn multiple_match_negative() {
// 		let extension = Extension {
// 			extensions: vec!["!pdf".into(), "doc".into(), "docx".into()],
// 		};
// 		let path = Resource::new::<_, PathBuf>("$HOME/Downloads/test.pdf", None).unwrap();
// 		let harness = ContextHarness::new();
// 		let context = harness.context();
// 		assert!(!extension.filter(&path, &context))
// 	}

// 	#[test]
// 	fn no_match() {
// 		let extension = Extension {
// 			extensions: vec!["pdf".into(), "doc".into(), "docx".into()],
// 		};
// 		let path = Resource::new::<_, PathBuf>("$HOME/Downloads/test.jpg", None).unwrap();
// 		let harness = ContextHarness::new();
// 		let context = harness.context();
// 		assert!(!extension.filter(&path, &context))
// 	}
// }
