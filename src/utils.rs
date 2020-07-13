use std::io::{Error, ErrorKind};
use std::path::Path;

/// # Arguments
/// * `path`: A reference to a std::path::PathBuf
/// # Return
/// Returns the stem and extension of `path` if they exist and can be parsed, otherwise returns an Error
pub(crate) fn get_stem_and_extension(path: &Path) -> Result<(&str, &str), Error> {
    let stem = path
        .file_stem()
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "file does not have a file stem (?)"))?
        .to_str()
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "cannot convert OsStr to &str"))?;
    let extension = path
        .extension()
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "file does not have an extension"))?
        .to_str()
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "cannot convert OsStr to str"))?;

    Ok((stem, extension))
}