use crate::{
	config::{
		context::ExecutionContext,
		filters::{regex::RegexSet, Filter},
	},
	resource::Resource,
	templates::template::Template,
};
use anyhow::Result;
use gag::Gag;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{panic::catch_unwind, path::Path, sync::Arc};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Content {
	#[serde(default)]
	contains: Vec<Template>,
	#[serde(default)]
	matches: RegexSet,
}

#[typetag::serde(name = "content")]
impl Filter for Content {
	fn templates(&self) -> Vec<&Template> {
		self.contains.iter().collect_vec()
	}

	fn filter(&self, res: &Resource, ctx: &ExecutionContext) -> bool {
		// Lazily get the content from the cache.
		let content_arc = ctx
			.services
			.content_cache
			.entry(res.path().to_path_buf())
			.or_try_insert_with(|| -> Result<Arc<String>> {
				let content = extract_content(res.path())?;
				Ok(Arc::new(content.unwrap_or_default()))
			})
			.ok()
			.map(|entry| entry.value().clone());

		if let Some(content) = content_arc {
			// The filter logic is updated to render each template before checking.
			let context = ctx
				.services
				.template_engine
				.context()
				.path(res.path())
				.root(res.root())
				.build(&ctx.services.template_engine);
			let contains_match = self.contains.is_empty()
				|| self.contains.iter().any(
					|template| match ctx.services.template_engine.render(template, &context).unwrap_or_default() {
						Some(pattern) => content.contains(&pattern),
						None => false,
					},
				);

			let regex_match = self.matches.is_empty() || self.matches.is_match(&content);

			contains_match && regex_match
		} else {
			false
		}
	}
}

/// This function now acts as a dispatcher, delegating to the appropriate
/// private helper based on the file's MIME type.
fn extract_content(path: &Path) -> Result<Option<String>> {
	let mime = mime_guess::from_path(path).first_or_text_plain();

	match (mime.type_().as_str(), mime.subtype().as_str()) {
		("text", _) => read_text(path),
		("application", "pdf") => read_pdf(path),
		_ => {
			tracing::debug!("No content extractor found for MIME type: {}", mime);
			Ok(None)
		}
	}
}

/// Extracts content from plain text files.
fn read_text(path: &Path) -> Result<Option<String>> {
	Ok(Some(std::fs::read_to_string(path)?))
}

/// Extracts content from PDF files, with panic handling.
fn read_pdf(path: &Path) -> Result<Option<String>> {
	let result = catch_unwind(|| {
		let _gag = Gag::stderr().unwrap();
		pdf_extract::extract_text(&path).ok()
	});

	match result {
		Ok(text) => Ok(text),
		Err(_) => {
			tracing::warn!(
				"Could not extract text from {}. The file may be malformed or it may have an unsupported encoding.",
				path.display()
			);
			Ok(None)
		}
	}
}
