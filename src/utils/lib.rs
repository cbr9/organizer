#[cfg(test)]
mod new_filepath {
    use std::{
        env,
        io::{
            Error,
            ErrorKind,
        },
        path::PathBuf,
    };

    use crate::{
        user_config::rules::actions::{
            ConflictOption,
            ConflictingFileOperation,
        },
        utils::new_filepath,
    };

    static WATCHING: bool = false;

    fn project_dir() -> PathBuf {
        // 'cargo test' must be run from the project directory, where Cargo.toml is
        // even if you run it from some other folder inside the project
        // 'cargo test' will move to the project root
        env::current_dir().unwrap()
    }

    fn tests_directory() -> PathBuf {
        project_dir().join("tests")
    }

    fn test_file_or_dir(filename: &str) -> PathBuf {
        tests_directory().join("files").join(filename)
    }

    #[test]
    fn rename_with_rename_conflict() -> Result<(), Error> {
        let action = ConflictingFileOperation::from(test_file_or_dir("test2.txt"));
        let file = test_file_or_dir("test1.txt");
        let new_path = new_filepath(&file, &action, WATCHING)?;
        let expected = PathBuf::from(format!(
            "{}/test2 (1).txt",
            action.to.parent().unwrap().to_str().unwrap()
        ));
        if new_path == expected {
            Ok(())
        } else {
            Err(Error::new(ErrorKind::Other, "filepath after rename is not as expected"))
        }
    }

    #[test]
    fn rename_with_rename_conflict_and_different_sep() -> Result<(), Error> {
        let mut action = ConflictingFileOperation::from(test_file_or_dir("test2.txt"));
        action.counter_separator = "-".to_string();
        let file = test_file_or_dir("test1.txt");
        let new_path = new_filepath(&file, &action, WATCHING)?;
        let expected = PathBuf::from(format!(
            "{}/test2-(1).txt",
            action.to.parent().unwrap().to_str().unwrap()
        ));
        if new_path == expected {
            Ok(())
        } else {
            Err(Error::new(ErrorKind::Other, "filepath after rename is not as expected"))
        }
    }

    #[test]
    fn move_with_rename_conflict() -> Result<(), Error> {
        let action = ConflictingFileOperation::from(test_file_or_dir("test_dir"));
        let file = test_file_or_dir("test1.txt");
        let new_path = new_filepath(&file, &action, WATCHING)?;
        let expected = PathBuf::from(format!("{}/test1 (1).txt", action.to.to_str().unwrap()));
        if new_path == expected {
            Ok(())
        } else {
            Err(Error::new(ErrorKind::Other, "filepath after rename is not as expected"))
        }
    }

    #[test]
    fn rename_with_overwrite_conflict() -> Result<(), Error> {
        let mut action = ConflictingFileOperation::from(test_file_or_dir("test2.txt"));
        action.if_exists = ConflictOption::Overwrite;
        let file = test_file_or_dir("test1.txt");
        let new_path = new_filepath(&file, &action, WATCHING)?;
        if new_path == action.to {
            Ok(())
        } else {
            Err(Error::new(ErrorKind::Other, "filepath after rename is not as expected"))
        }
    }
    #[test]
    fn move_with_overwrite_conflict() -> Result<(), Error> {
        let mut action = ConflictingFileOperation::from(test_file_or_dir("test_dir"));
        action.if_exists = ConflictOption::Overwrite;
        let file = test_file_or_dir("test1.txt");
        let new_path = new_filepath(&file, &action, WATCHING)?;
        let expected = PathBuf::from(format!("{}/test1.txt", action.to.to_str().unwrap()));
        if new_path == expected {
            Ok(())
        } else {
            Err(Error::new(ErrorKind::Other, "filepath after rename is not as expected"))
        }
    }

    #[test]
    fn rename_with_skip_conflict() -> Result<(), Error> {
        let mut action = ConflictingFileOperation::from(test_file_or_dir("test2.txt"));
        action.if_exists = ConflictOption::Skip;
        let original = test_file_or_dir("test1.txt");
        let new = new_filepath(&original, &action, WATCHING)?;
        if original == new {
            Ok(())
        } else {
            Err(Error::new(ErrorKind::Other, "filepath after rename is not as expected"))
        }
    }

    #[test]
    fn move_with_skip_conflict() -> Result<(), Error> {
        let mut action = ConflictingFileOperation::from(test_file_or_dir("test_dir"));
        action.if_exists = ConflictOption::Skip;
        let original = test_file_or_dir("test1.txt");
        let new = new_filepath(&original, &action, WATCHING)?;
        if original == new {
            Ok(())
        } else {
            Err(Error::new(ErrorKind::Other, "filepath after move is not as expected"))
        }
    }
}

#[cfg(test)]
mod expand_env_var {
    use std::{
        io::{
            Error,
            ErrorKind,
        },
        path::PathBuf,
    };

    use dirs::home_dir;

    use crate::user_config::rules::deserialize::PathExpandable;

    #[test]
    fn home() -> Result<(), Error> {
        let tested = PathBuf::from("$HOME/Documents").expand();
        let expected = home_dir().unwrap().join("Documents");
        if tested == expected {
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::InvalidData,
                "the environment variable wasn't properly expanded",
            ))
        }
    }
}
