use std::{
	borrow::Cow,
	io::{Error, ErrorKind, Result},
	path::Path,
};

use crate::config::{ConflictOption, Sep};

pub trait Update {
	fn update(&self, if_exists: &ConflictOption, sep: &Sep) -> Result<Cow<Path>>;
}

impl Update for Path {
	///  When trying to rename a file to a path that already exists, calling update() on the
	///  target path will return a new valid path.
	///  # Args
	/// * `if_exists`: option to resolve the naming conflict
	/// * `sep`: if `if_exists` is set to rename, `sep` will go between the filename and the added counter
	/// * `is_watching`: whether this function is being run from a watcher or not
	/// # Return
	/// This function will return `Some(new_path)` if `if_exists` is not set to skip, otherwise it returns `None`
	fn update(&self, if_exists: &ConflictOption, sep: &Sep) -> Result<Cow<Path>> {
		debug_assert!(self.exists());

		match if_exists {
			ConflictOption::Skip => Err(Error::from(ErrorKind::AlreadyExists)),
			ConflictOption::Overwrite => Ok(Cow::Borrowed(self)),
			ConflictOption::Rename => {
				let extension = self.extension().unwrap_or_default().to_string_lossy();
				let stem = match self.file_stem() {
					Some(stem) => stem.to_string_lossy(),
					None => return Err(Error::from(ErrorKind::InvalidInput)),
				};
				// let (stem, extension) = path::get_stem_and_extension(&self);
				let mut new = self.to_path_buf();
				let mut n = 1;
				while new.exists() {
					let new_filename = format!("{}{}({:?}).{}", stem, sep.as_str(), n, extension);
					new.set_file_name(new_filename);
					n += 1;
				}
				Ok(Cow::Owned(new))
			}
		}
	}
}
