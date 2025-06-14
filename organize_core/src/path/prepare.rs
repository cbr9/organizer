use path_clean::PathClean;
use std::path::{PathBuf, MAIN_SEPARATOR};

use anyhow::Result;

use crate::{
	config::{
		actions::common::{resolve_naming_conflict, ConflictOption},
		variables::Variable,
	},
	resource::Resource,
	templates::{template::Template, TemplateEngine},
};

use super::expand::Expand;

pub fn prepare_target_path(
	if_exists: &ConflictOption,
	resource: &Resource,
	dest: &Template,
	with_extension: bool,
	template_engine: &TemplateEngine,
	variables: &[Box<dyn Variable>],
) -> Result<Option<PathBuf>> {
	let context = TemplateEngine::new_context(resource, variables);
	let rendered_dest = template_engine.render(dest, &context)?;
	let mut to = PathBuf::from(rendered_dest).expand_user().clean();

	let path = &resource.path;

	if to.extension().is_none() || to.is_dir() || to.to_string_lossy().ends_with(MAIN_SEPARATOR) {
		if with_extension {
			let filename = path.file_name();
			if filename.is_none() {
				// This can happen if the path is something like "." or ".."
				return Ok(None);
			}
			to.push(filename.unwrap());
		} else {
			let stem = path.file_stem();
			if stem.is_none() {
				return Ok(None);
			}
			to.push(stem.unwrap())
		}
	}

	let resolved = resolve_naming_conflict(if_exists, &to);
	Ok(resolved)
}
