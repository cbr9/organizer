use std::{
    env,
    io::Result,
    path::{Path, PathBuf},
};

use crate::{
    subcommands::run::resolve_conflict,
    user_config::rules::{
        actions::{ConflictOption, Sep},
        filters::{Filename, Filters},
    },
    WATCHING,
};
use std::{
    borrow::Cow,
    io::{Error, ErrorKind},
    sync::atomic::Ordering,
};

pub mod lib;

pub trait MatchesFilters {
    fn matches_filters(&self, filters: &Filters) -> bool;
    fn is_hidden(&self) -> bool;
}

impl MatchesFilters for PathBuf {
    fn matches_filters(&self, filters: &Filters) -> bool {
        if *filters == Filters::default() {
            // empty filters
            return false;
        }
        let extension = self.extension().unwrap_or_default().to_str().unwrap_or_default();
        let temporary_file_extensions = ["crdownload", "part", "tmp", "download"];
        if !extension.is_empty() && temporary_file_extensions.contains(&extension) {
            return false;
        }

        let as_str = self.to_str().unwrap();
        if !filters.regex.to_string().is_empty() && !filters.regex.is_match(&as_str) {
            return false;
        }

        let Filename {
            startswith,
            endswith,
            contains,
            case_sensitive,
        } = &filters.filename;

        let mut filename = self.file_name().unwrap().to_str().unwrap().to_string();
        if !case_sensitive {
            filename = filename.to_lowercase();
        }
        if !startswith.is_empty() && !filename.starts_with(startswith) {
            return false;
        }
        if !endswith.is_empty() && !filename.ends_with(endswith) {
            return false;
        }
        if !contains.is_empty() && !filename.contains(contains) {
            return false;
        }
        if !filters.extensions.is_empty() && !filters.extensions.contains(&extension.to_string()) {
            return false;
        }
        true
    }

    fn is_hidden(&self) -> bool {
        self.file_name().unwrap().to_str().unwrap().starts_with('.')
    }
}

pub trait Update {
    fn update(&mut self, if_exists: &ConflictOption, sep: &Sep) -> Result<()>;
}

impl Update for Cow<'_, Path> {
    ///  When trying to rename a file to a path that already exists, calling update() on the
    ///  target path will return a new valid path.
    ///  # Args
    /// * `if_exists`: option to resolve the naming conflict
    /// * `sep`: if `if_exists` is set to rename, `sep` will go between the filename and the added counter
    /// * `is_watching`: whether this function is being run from a watcher or not
    /// # Return
    /// This function will return `Some(new_path)` if `if_exists` is not set to skip, otherwise it returns `None`
    fn update(&mut self, if_exists: &ConflictOption, sep: &Sep) -> Result<()> {
        debug_assert!(self.exists());

        match if_exists {
            ConflictOption::Skip => Err(Error::from(ErrorKind::AlreadyExists)),
            ConflictOption::Overwrite => Ok(()),
            ConflictOption::Rename => {
                let (stem, extension) = get_stem_and_extension(&self);
                let mut n = 1;
                while self.exists() {
                    let new_filename = format!("{}{}({:?}).{}", stem, sep.as_str(), n, extension);
                    self.to_mut().set_file_name(new_filename);
                    n += 1;
                }
                Ok(())
            }
            ConflictOption::Ask => {
                debug_assert_ne!(ConflictOption::default(), ConflictOption::Ask);
                let if_exists = if WATCHING.load(Ordering::SeqCst) {
                    Default::default()
                } else {
                    resolve_conflict(&self)
                };
                self.update(&if_exists, sep)
            }
        }
    }
}

pub trait Expandable {
    fn expand_user(&self) -> PathBuf;
    fn expand_vars(&self) -> PathBuf;
}

impl Expandable for PathBuf {
    fn expand_user(&self) -> Self {
        let str = self.to_str().unwrap().to_string();
        Self::from(str.replace("~", "$HOME"))
    }

    fn expand_vars(&self) -> Self {
        self.components()
            .map(|component| {
                let component: &Path = component.as_ref();
                let component = component.to_str().unwrap();
                if component.starts_with('$') {
                    env::var(component.replace('$', ""))
                        .unwrap_or_else(|_| panic!("error: environment variable '{}' could not be found", component))
                } else {
                    component.to_string()
                }
            })
            .collect()
    }
}

/// # Arguments
/// * `path`: A reference to a std::path::PathBuf
/// # Return
/// Returns the stem and extension of `path` if they exist and can be parsed, otherwise returns an Error
fn get_stem_and_extension(path: &Path) -> (String, String) {
    let stem = path.file_stem().unwrap().to_str().unwrap().to_string();
    let extension = path.extension().unwrap_or_default().to_str().unwrap().to_string();

    (stem, extension)
}
