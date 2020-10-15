mod lib;

use crate::{
    user_config::rules::filters::Filters,
    utils::expand_env_vars,
};
use regex::Regex;
use std::{
    io::Error,
    path::{
        Path,
        PathBuf,
    },
};

pub struct File {
    pub filename: String,
    pub stem: String,
    pub extension: String,
    pub path: PathBuf,
    pub is_hidden: bool,
}

impl From<PathBuf> for File {
    fn from(path: PathBuf) -> Self {
        let (stem, extension) = get_stem_and_extension(&path).unwrap();
        let filename = String::from(path.file_name().unwrap().to_str().unwrap());
        File {
            is_hidden: filename.starts_with('.'),
            filename,
            stem,
            extension,
            path: expand_env_vars(&path),
        }
    }
}

impl From<&Path> for File {
    fn from(path: &Path) -> Self {
        let (stem, extension) = get_stem_and_extension(path).unwrap();
        let filename = String::from(path.file_name().unwrap().to_str().unwrap());
        File {
            is_hidden: filename.starts_with('.'),
            filename,
            stem,
            extension,
            path: expand_env_vars(path),
        }
    }
}

impl From<&str> for File {
    fn from(path: &str) -> Self {
        let path = expand_env_vars(Path::new(path));
        Self::from(path)
    }
}

impl File {
    pub fn matches_filters(&self, filters: &Filters) -> bool {
        // TODO test this function
        let temporary_file_extensions = ["crdownload", "part", "tmp", "download"];
        if temporary_file_extensions.contains(&self.extension.as_str()) {
            return false;
        }

        let path = self.path.to_str().unwrap();
        if !filters.regex.is_empty() {
            let regex = Regex::new(&filters.regex).unwrap();
            if regex.is_match(&path) {
                return true;
            }
        }
        if !filters.filename.is_empty() && self.filename == filters.filename {
            return true;
        }
        if !filters.extensions.is_empty() {
            for extension in filters.extensions.iter() {
                if self.extension.eq(extension) {
                    return true;
                }
            }
        }
        false
    }
}

/// # Arguments
/// * `path`: A reference to a std::path::PathBuf
/// # Return
/// Returns the stem and extension of `path` if they exist and can be parsed, otherwise returns an Error
pub fn get_stem_and_extension(path: &Path) -> Result<(String, String), Error> {
    let stem = path.file_stem().unwrap().to_str().unwrap().to_owned();
    let extension = path.extension().unwrap_or_default().to_str().unwrap().to_owned();

    Ok((stem, extension))
}
