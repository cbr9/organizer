use crate::{
	config::{context::ExecutionContext, filters::Filter},
	resource::Resource,
	templates::template::Template,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{panic::catch_unwind, path::Path, sync::Arc};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Content {
	pub contains: Template,
}

#[typetag::serde(name = "content")]
impl Filter for Content {
	fn templates(&self) -> Vec<&Template> {
		vec![&self.contains]
	}

	fn filter(&self, res: &Resource, ctx: &ExecutionContext) -> bool {
		let context = ctx.services.template_engine.new_context(res);
		if let Ok(Some(contains)) = ctx.services.template_engine.render(&self.contains, &context) {
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

			return content_arc.is_some_and(|c| c.contains(&contains));
		}

		false
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
		let bytes = std::fs::read(path).ok()?;
		pdf_extract::extract_text_from_mem(&bytes).ok()
	});

	match result {
		Ok(text) => Ok(text),
		Err(_) => {
			tracing::error!(
				"The `pdf-extract` library panicked while processing: {}. The file may be severely malformed.",
				path.display()
			);
			Ok(None)
		}
	}
}
