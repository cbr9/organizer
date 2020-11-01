pub mod expand;
pub mod is_hidden;
pub mod update;

#[cfg(test)]
pub mod helpers {
    // TODO: Refactor these helpers
    use crate::user_config::rules::actions::io_action::Sep;
    use std::{
        env,
        io::{Error, ErrorKind, Result},
        ops::Deref,
        path::{Path, PathBuf},
    };

    pub fn project_dir() -> PathBuf {
        // 'cargo test' must be run from the project directory, where Cargo.toml is
        // even if you run it from some other folder inside the project
        // 'cargo test' will move to the project root
        env::current_dir().unwrap()
    }

    pub fn tests_dir() -> PathBuf {
        project_dir().join("tests")
    }

    pub fn test_file_or_dir(filename: &str) -> PathBuf {
        tests_dir().join("files").join(filename)
    }

    pub fn expected_path(file: &impl AsRef<Path>, sep: &Sep) -> Result<PathBuf> {
        let extension = file
            .as_ref()
            .extension()
            .unwrap_or_default()
            .to_string_lossy();
        let stem = match file.as_ref().file_stem() {
            Some(stem) => stem.to_string_lossy(),
            None => return Err(Error::from(ErrorKind::InvalidInput)),
        };
        let parent = file.as_ref().parent().unwrap();
        Ok(parent.join(format!("{}{}(1).{}", stem, sep.deref(), extension)))
    }
}
