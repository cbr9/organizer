



// #[cfg(test)]
// mod tests {
// 	use super::*;
// 	use crate::{
// 		config::{context::ContextHarness, variables::simple::SimpleVariable},
// 		templates::Templater,
// 	};
// 	use path_clean::PathClean;
// 	use pretty_assertions::assert_eq;
// 	use std::{
// 		convert::{TryFrom, TryInto},
// 		fs,
// 		path::Path,
// 	};
// 	use tempfile::{tempdir, NamedTempFile};

// 	// Helper function to create a Resource from a tempfile.
// 	fn create_resource(file: &NamedTempFile) -> Resource {
// 		Resource::new(file.path(), Path::new("/tmp")).unwrap()
// 	}

// 	/// Tests the scenario where the destination is a directory and the original file's
// 	/// full name (stem and extension) should be appended to the destination path.
// 	#[test]
// 	fn destination_is_dir_with_extension() {
// 		let temp_file = NamedTempFile::new().unwrap();
// 		let resource = create_resource(&temp_file);
// 		let dest_template = Template::try_from("/tmp/test_dir/").unwrap();

// 		let mut harness = ContextHarness::new();
// 		harness.services.templater.add_template(&dest_template).unwrap();
// 		let ctx = harness.context(); // Create a dry run context

// 		let result = prepare_target_path(&ConflictResolution::Skip, &resource, &dest_template, true, &ctx).unwrap();

// 		let expected_filename = temp_file.path().file_name().unwrap();
// 		let expected_path = PathBuf::from("/tmp/test_dir/").join(expected_filename).clean();
// 		assert_eq!(result.map(|r| r.as_path().clean()), Some(expected_path));
// 	}

// 	/// Tests the scenario where the destination is a directory and only the file's
// 	/// stem (filename without extension) should be appended to the destination path.
// 	#[test]
// 	fn destination_is_dir_without_extension() {
// 		let temp_dir = tempdir().unwrap();
// 		let temp_file = NamedTempFile::new_in(&temp_dir).unwrap();
// 		let resource = create_resource(&temp_file);
// 		let dest_dir = temp_dir.path().join("test_dir");
// 		fs::create_dir_all(&dest_dir).unwrap();
// 		let dest_template = Template::try_from(dest_dir.to_str().unwrap()).unwrap();

// 		let mut harness = ContextHarness::new();
// 		harness.services.templater.add_template(&dest_template).unwrap();
// 		let ctx = harness.context(); // dry run

// 		let result = prepare_target_path(&ConflictResolution::Skip, &resource, &dest_template, false, &ctx).unwrap();

// 		let expected_filename = temp_file.path().file_stem().unwrap();
// 		let expected_path = dest_dir.join(expected_filename);
// 		assert_eq!(result.map(|r| r.to_path_buf()), Some(expected_path));
// 	}

// 	/// Tests if the function correctly handles a destination path that is rendered
// 	/// from a template variable.
// 	#[test]
// 	fn destination_with_templated_variable() {
// 		let temp_file = NamedTempFile::new().unwrap();
// 		let resource = create_resource(&temp_file);
// 		let dest_template = Template::try_from("/tmp/{{ name }}/").unwrap();

// 		let variable = SimpleVariable {
// 			name: "name".into(),
// 			value: "rendered_dir".try_into().unwrap(),
// 		};
// 		let mut harness = ContextHarness::new();
// 		let template_engine = Templater::new(&vec![Box::new(variable)]);
// 		harness.services.templater = template_engine;
// 		harness.services.templater.add_template(&dest_template).unwrap();
// 		let ctx = harness.context(); // dry run

// 		let result = prepare_target_path(&ConflictResolution::Skip, &resource, &dest_template, true, &ctx).unwrap();

// 		let expected_filename = temp_file.path().file_name().unwrap();
// 		let expected_path = PathBuf::from("/tmp/rendered_dir").join(expected_filename);
// 		assert_eq!(result.map(|r| r.as_path().clean()), Some(expected_path.clean()));
// 	}

// 	/// Tests the `Skip` conflict resolution strategy.
// 	/// When the target file already exists, the function should return `None`.
// 	#[test]
// 	fn conflict_skip_existing_file() {
// 		let temp_file = NamedTempFile::new().unwrap(); // This file exists
// 		let resource = create_resource(&temp_file);
// 		let dest_template = Template::try_from(temp_file.path().to_str().unwrap()).unwrap();

// 		let mut harness = ContextHarness::new();
// 		harness.services.templater.add_template(&dest_template).unwrap();
// 		let ctx = harness.context(); // dry run

// 		let result = prepare_target_path(&ConflictResolution::Skip, &resource, &dest_template, true, &ctx).unwrap();

// 		// Expect None because the file exists and the conflict option is Skip
// 		assert!(result.is_none());
// 	}

// 	/// Tests the `Overwrite` conflict resolution strategy.
// 	/// When the target file already exists, the function should return the path to that same file.
// 	#[test]
// 	fn conflict_overwrite_existing_file() {
// 		let temp_file = NamedTempFile::new().unwrap();
// 		let resource = create_resource(&temp_file);
// 		let dest_template = Template::try_from(temp_file.path().to_str().unwrap()).unwrap();

// 		let mut harness = ContextHarness::new();
// 		harness.services.templater.add_template(&dest_template).unwrap();
// 		let ctx = harness.context(); // dry run

// 		let result = prepare_target_path(&ConflictResolution::Overwrite, &resource, &dest_template, true, &ctx).unwrap();

// 		assert_eq!(result.map(|r| r.to_path_buf()), Some(temp_file.path().to_path_buf()));
// 	}

// 	/// Tests the `Rename` conflict resolution strategy.
// 	#[test]
// 	fn conflict_rename_existing_file() {
// 		let temp_file = NamedTempFile::with_suffix(".txt").unwrap();
// 		let resource = create_resource(&temp_file);
// 		let dest_template = Template::try_from(temp_file.path().to_str().unwrap()).unwrap();

// 		let mut harness = ContextHarness::new();
// 		harness.services.templater.add_template(&dest_template).unwrap();
// 		let ctx = harness.context(); // dry run

// 		let result = prepare_target_path(&ConflictResolution::Rename, &resource, &dest_template, true, &ctx).unwrap();

// 		let file_stem = temp_file.path().file_stem().unwrap().to_str().unwrap();
// 		let extension = temp_file.path().extension().unwrap().to_str().unwrap();
// 		let expected_path = temp_file.path().with_file_name(format!("{} (1).{}", file_stem, extension));

// 		assert_eq!(result.map(|r| r.to_path_buf()), Some(expected_path));
// 	}

// 	/// Tests the case where the destination is just a filename.
// 	#[test]
// 	fn destination_is_filename_only() {
// 		let temp_file = NamedTempFile::new().unwrap();
// 		let resource = create_resource(&temp_file);
// 		let dest_template = Template::try_from("just_a_filename.txt").unwrap();

// 		let mut harness = ContextHarness::new();
// 		harness.services.templater.add_template(&dest_template).unwrap();
// 		let ctx = harness.context(); // dry run

// 		let result = prepare_target_path(&ConflictResolution::Skip, &resource, &dest_template, true, &ctx).unwrap();

// 		assert_eq!(result.map(|r| r.to_path_buf()), Some(PathBuf::from("just_a_filename.txt")));
// 	}

// 	/// Tests tilde expansion for the user's home directory.
// 	#[test]
// 	fn destination_expands_user_home() {
// 		let temp_file = NamedTempFile::new().unwrap();
// 		let resource = create_resource(&temp_file);
// 		let dest_template = Template::try_from("~/test_dir/output.txt").unwrap();

// 		let mut harness = ContextHarness::new();
// 		harness.services.templater.add_template(&dest_template).unwrap();
// 		let ctx = harness.context(); // dry run

// 		let result = prepare_target_path(&ConflictResolution::Skip, &resource, &dest_template, true, &ctx).unwrap();

// 		let home_dir = dirs::home_dir().unwrap();
// 		let expected_path = home_dir.join("test_dir/output.txt");
// 		assert_eq!(result.map(|r| r.to_path_buf()), Some(expected_path));
// 	}

// 	/// Tests Rename when multiple conflicting files exist.
// 	#[test]
// 	fn conflict_rename_multiple_existing_files() {
// 		let dir = tempdir().unwrap();
// 		let original_path = dir.path().join("file.txt");
// 		fs::write(&original_path, "original").unwrap();
// 		fs::write(dir.path().join("file (1).txt"), "first conflict").unwrap();
// 		let resource = Resource::new(&original_path, dir.path()).unwrap();
// 		let dest_template = Template::try_from(original_path.to_str().unwrap()).unwrap();

// 		let mut harness = ContextHarness::new();
// 		harness.services.templater.add_template(&dest_template).unwrap();
// 		let ctx = harness.context(); // dry run

// 		let result = prepare_target_path(&ConflictResolution::Rename, &resource, &dest_template, true, &ctx).unwrap();

// 		let expected_path = dir.path().join("file (2).txt");
// 		assert_eq!(result.map(|r| r.to_path_buf()), Some(expected_path));
// 	}

// 	/// Tests that an empty rendered template results in `None`.
// 	#[test]
// 	fn destination_template_renders_to_empty() {
// 		let temp_file = NamedTempFile::new().unwrap();
// 		let resource = create_resource(&temp_file);
// 		let dest_template = Template::try_from("{{ empty_var }}").unwrap();
// 		let variable = SimpleVariable {
// 			name: "empty_var".into(),
// 			value: "".try_into().unwrap(),
// 		};

// 		let mut harness = ContextHarness::new();
// 		let template_engine = Templater::new(&vec![Box::new(variable)]);
// 		harness.services.templater = template_engine;
// 		harness.services.templater.add_template(&dest_template).unwrap();
// 		let ctx = harness.context(); // dry run

// 		let result = prepare_target_path(&ConflictResolution::Skip, &resource, &dest_template, true, &ctx).unwrap();
// 		assert!(result.is_none());
// 	}
// }
