use path_clean::PathClean;
use std::path::{PathBuf, MAIN_SEPARATOR};

use anyhow::Result;

use crate::{
	config::actions::common::{resolve_naming_conflict, ConflictOption},
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
) -> Result<Option<PathBuf>> {
	let context = template_engine.context(resource);
	let rendered_dest = template_engine.render(dest, &context)?;
	if rendered_dest.is_none() {
		return Ok(None);
	}
	let rendered_dest = rendered_dest.unwrap();
	let mut to = PathBuf::from(rendered_dest).expand_user();

	let path = &resource.path();

	if to.is_dir() || to.to_string_lossy().ends_with(MAIN_SEPARATOR) || to.to_string_lossy().ends_with('/') {
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

	let resolved = resolve_naming_conflict(if_exists, &to).map(|p| p.clean());
	Ok(resolved)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{config::variables::simple::SimpleVariable, templates::TemplateEngine};
	use pretty_assertions::assert_eq;
	use std::{fs, path::Path};
	use tempfile::{tempdir, NamedTempFile};

	// Helper function to create a default TemplateEngine for tests.
	fn create_template_engine() -> TemplateEngine {
		TemplateEngine::default()
	}

	// Helper function to create a Resource from a tempfile.
	fn create_resource(file: &NamedTempFile) -> Resource {
		Resource::new(file.path(), Path::new("/tmp")).unwrap()
	}

	/// Tests the scenario where the destination is a directory and the original file's
	/// full name (stem and extension) should be appended to the destination path.
	#[test]
	fn destination_is_dir_with_extension() {
		let mut template_engine = create_template_engine();
		let temp_file = NamedTempFile::new().unwrap();
		let resource = create_resource(&temp_file);
		let dest_template = Template::from("/tmp/test_dir/");
		template_engine.add_template(&dest_template).unwrap();

		let result = prepare_target_path(
			&ConflictOption::Skip,
			&resource,
			&dest_template,
			true, // with_extension is true
			&template_engine,
		)
		.unwrap()
		.map(|p| p.clean());

		let expected_filename = temp_file.path().file_name().unwrap();
		let expected_path = PathBuf::from("/tmp/test_dir/").join(expected_filename).clean();
		assert_eq!(result, Some(expected_path));
	}

	/// Tests the scenario where the destination is a directory and only the file's
	/// stem (filename without extension) should be appended to the destination path.
	#[test]
	fn destination_is_dir_without_extension() {
		let mut template_engine = create_template_engine();
		let temp_dir = tempdir().unwrap();
		let temp_file = NamedTempFile::new_in(&temp_dir).unwrap();
		let resource = create_resource(&temp_file);
		let dest_dir = temp_dir.path().join("test_dir");
		fs::create_dir_all(&dest_dir).unwrap();

		let dest_template = Template::from(dest_dir.to_str().unwrap());
		template_engine.add_template(&dest_template).unwrap();

		let result = prepare_target_path(
			&ConflictOption::Skip,
			&resource,
			&dest_template,
			false, // with_extension is false
			&template_engine,
		)
		.unwrap();

		let expected_filename = temp_file.path().file_stem().unwrap();
		let expected_path = dest_dir.join(expected_filename);
		assert_eq!(result, Some(expected_path));
	}

	/// Tests if the function correctly handles a destination path that is rendered
	/// from a template variable.
	#[test]
	fn destination_with_templated_variable() {
		let temp_file = NamedTempFile::new().unwrap();
		let resource = create_resource(&temp_file);
		let dest_template = Template::from("/tmp/{{ name }}/");

		// Create a variable and a template engine that knows about it.
		let variable = SimpleVariable {
			name: "name".into(),
			value: "rendered_dir".into(),
		};
		let mut template_engine = TemplateEngine::new(&vec![Box::new(variable)]);
		// The template being rendered must be added to the engine
		template_engine.add_template(&dest_template).unwrap();

		let result = prepare_target_path(&ConflictOption::Skip, &resource, &dest_template, true, &template_engine).unwrap();

		let expected_filename = temp_file.path().file_name().unwrap();
		let expected_path = PathBuf::from("/tmp/rendered_dir").join(expected_filename);
		assert_eq!(result.map(|p| p.clean()), Some(expected_path.clean()));
	}

	/// Tests the `Skip` conflict resolution strategy.
	/// When the target file already exists, the function should return `None`.
	#[test]
	fn conflict_skip_existing_file() {
		let mut template_engine = create_template_engine();
		let temp_file = NamedTempFile::new().unwrap(); // This file exists
		let resource = create_resource(&temp_file);
		let dest_template = Template::from(temp_file.path().to_str().unwrap());
		template_engine.add_template(&dest_template).unwrap();

		let result = prepare_target_path(&ConflictOption::Skip, &resource, &dest_template, true, &template_engine).unwrap();

		// Expect None because the file exists and the conflict option is Skip
		assert_eq!(result, None);
	}

	/// Tests the `Overwrite` conflict resolution strategy.
	/// When the target file already exists, the function should return the path to that same file.
	#[test]
	fn conflict_overwrite_existing_file() {
		let mut template_engine = create_template_engine();
		let temp_file = NamedTempFile::new().unwrap();
		let resource = create_resource(&temp_file);
		let dest_template = Template::from(temp_file.path().to_str().unwrap());
		template_engine.add_template(&dest_template).unwrap();

		let result = prepare_target_path(&ConflictOption::Overwrite, &resource, &dest_template, true, &template_engine).unwrap();

		// Expect the same path because we are overwriting
		assert_eq!(result, Some(temp_file.path().to_path_buf()));
	}

	/// Tests the `Rename` conflict resolution strategy.
	/// When the target file already exists, the function should return a new, non-conflicting path
	/// with a counter (e.g., "file (1).txt").
	#[test]
	fn conflict_rename_existing_file() {
		let mut template_engine = create_template_engine();
		// Create a file with a known extension to test renaming logic
		let temp_file = NamedTempFile::with_suffix(".txt").unwrap();
		let resource = create_resource(&temp_file);
		let dest_template = Template::from(temp_file.path().to_str().unwrap());
		template_engine.add_template(&dest_template).unwrap();

		let result = prepare_target_path(&ConflictOption::Rename, &resource, &dest_template, true, &template_engine).unwrap();

		let file_stem = temp_file.path().file_stem().unwrap().to_str().unwrap();
		let extension = temp_file.path().extension().unwrap().to_str().unwrap();
		let expected_path = temp_file.path().with_file_name(format!("{} (1).{}", file_stem, extension));

		// Expect a new path with " (1)" appended
		assert_eq!(result, Some(expected_path));
	}

	/// Tests the case where the destination is just a filename, implying the current
	/// directory as the parent.
	#[test]
	fn destination_is_filename_only() {
		let mut template_engine = create_template_engine();
		let temp_file = NamedTempFile::new().unwrap();
		let resource = create_resource(&temp_file);
		let dest_template = Template::from("just_a_filename.txt");
		template_engine.add_template(&dest_template).unwrap();

		let result = prepare_target_path(&ConflictOption::Skip, &resource, &dest_template, true, &template_engine).unwrap();

		assert_eq!(result, Some(PathBuf::from("just_a_filename.txt")));
	}

	/// Tests the tilde expansion for the user's home directory.
	#[test]
	fn destination_expands_user_home() {
		let mut template_engine = create_template_engine();
		let temp_file = NamedTempFile::new().unwrap();
		let resource = create_resource(&temp_file);
		let dest_template = Template::from("~/test_dir/output.txt");
		template_engine.add_template(&dest_template).unwrap();

		let result = prepare_target_path(&ConflictOption::Skip, &resource, &dest_template, true, &template_engine).unwrap();

		let home_dir = dirs::home_dir().unwrap();
		let expected_path = home_dir.join("test_dir/output.txt");
		assert_eq!(result, Some(expected_path));
	}

	/// Tests the Rename conflict option when multiple files with the same name exist,
	/// requiring incrementing the counter beyond 1.
	#[test]
	fn conflict_rename_multiple_existing_files() {
		let mut template_engine = create_template_engine();
		let dir = tempfile::tempdir().unwrap();

		// Create the original file and a conflicting file
		let original_path = dir.path().join("file.txt");
		fs::write(&original_path, "original").unwrap();
		fs::write(dir.path().join("file (1).txt"), "first conflict").unwrap();

		let resource = Resource::new(&original_path, dir.path()).unwrap();
		let dest_template = Template::from(original_path.to_str().unwrap());
		template_engine.add_template(&dest_template).unwrap();

		// Create the first renamed file to force the next one to be (2)
		fs::write(dir.path().join(format!("file (1).txt")), "existing rename").unwrap();

		let result = prepare_target_path(&ConflictOption::Rename, &resource, &dest_template, true, &template_engine).unwrap();

		// The next available name should be "file (2).txt"
		let expected_path = dir.path().join("file (2).txt");
		assert_eq!(result, Some(expected_path));
	}

	/// Tests that if a template renders to an empty string, the path preparation
	/// does not panic and returns a reasonable result (the original filename in the current dir).
	#[test]
	fn destination_template_renders_to_empty() {
		let temp_file = NamedTempFile::new().unwrap();
		let resource = create_resource(&temp_file);

		let dest_template = Template::from("{{ empty_var }}");

		let variable = SimpleVariable {
			name: "empty_var".into(),
			value: "".into(),
		};
		let mut template_engine = TemplateEngine::new(&vec![Box::new(variable)]);
		template_engine.add_template(&dest_template).unwrap();

		let result = prepare_target_path(&ConflictOption::Skip, &resource, &dest_template, true, &template_engine).unwrap();
		assert_eq!(result, None);
	}
}
