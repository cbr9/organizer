#[cfg(test)]
pub mod vars {
    use std::{
        env,
        io::{Error, ErrorKind, Result},
        path::PathBuf,
    };

    use crate::{
        path::{get_stem_and_extension, Expandable},
        user_config::rules::actions::Sep,
    };
    use dirs::home_dir;
    use std::{borrow::Borrow, ops::Deref, path::Path};

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

    pub fn expected_path(file: &impl Borrow<Path>, sep: &Sep) -> PathBuf {
        let (stem, extension) = get_stem_and_extension(file);
        let parent = file.borrow().parent().unwrap();
        parent.join(format!("{}{}(1).{}", stem, sep.deref(), extension))
    }

    #[test]
    fn home() -> Result<()> {
        let tested = PathBuf::from("$HOME/Documents");
        let expected = home_dir().unwrap().join("Documents");
        if tested.expand_vars() == expected {
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::InvalidData,
                "the environment variable wasn't properly expanded",
            ))
        }
    }

    #[test]
    fn new_var() -> Result<()> {
        env::set_var("PROJECT_DIR", project_dir());
        let tested = PathBuf::from("$PROJECT_DIR/tests");
        if tested.expand_vars() == tests_dir() {
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::InvalidData,
                "the environment variable wasn't properly expanded",
            ))
        }
    }

    #[test]
    #[should_panic]
    fn non_existing_var() {
        let var = "PROJECT_DIR_2";
        assert!(
            env::var(var).is_err(),
            "PROJECT_DIR should not be a valid environment variable for this test"
        );
        let tested = PathBuf::from(format!("${}/tests", var));
        tested.expand_vars();
    }
}
// #[cfg(test)]
// mod filters {
//     use crate::{path::IsHidden, user_config::rules::filters::Filters};
//     use std::{
//         io::{Error, ErrorKind, Result},
//         path::PathBuf,
//     };
//
//     #[test]
//     fn test_temporary_files() -> Result<()> {
//         let crdownload = PathBuf::from("$HOME/Downloads/unsplash.jpg.crdownload");
//         let tmp = PathBuf::from("$HOME/Downloads/unsplash.jpg.tmp");
//         let part = PathBuf::from("$HOME/Downloads/unsplash.jpg.part");
//         let download = PathBuf::from("$HOME/Downloads/unsplash.jpg.crdownload");
//         let filters = Filters::default();
//         for file in [crdownload, tmp, part, download].iter() {
//             if file.matches_filters(&filters) {
//                 return Err(Error::new(ErrorKind::Other, "temporary file matched filters"));
//             }
//         }
//         Ok(())
//     }
//
//     #[test]
//     fn test_filters_extensions() -> Result<()> {
//         let file = PathBuf::from("/home/cabero/Documents/matricula.pdf");
//         let mut filters = Filters::default();
//         filters.extensions.push("pdf".to_string());
//         if file.matches_filters(&filters) {
//             Ok(())
//         } else {
//             Err(Error::new(
//                 ErrorKind::Other,
//                 "file did not match the filters correctly".to_string(),
//             ))
//         }
//     }
//     #[test]
//     fn test_filters_empty_filters() -> Result<()> {
//         let file = PathBuf::from("/home/cabero/Documents/matricula.pdf");
//         let filters = Filters::default();
//         if file.matches_filters(&filters) {
//             Err(Error::new(
//                 ErrorKind::Other,
//                 "file matched an empty set of filters".to_string(),
//             ))
//         } else {
//             Ok(())
//         }
//     }
//     #[test]
//     fn test_filters_filename_startswith() -> Result<()> {
//         let file = PathBuf::from("/home/cabero/Documents/matricula.pdf");
//         let mut filters = Filters::default();
//         filters.filename.startswith = "matricula".into();
//         if file.matches_filters(&filters) {
//             // the default regex is an empty one, so it should match everything
//             // but we check for this possibility before trying to match
//             Ok(())
//         } else {
//             Err(Error::new(
//                 ErrorKind::Other,
//                 "file did not match the filters correctly".to_string(),
//             ))
//         }
//     }
//     #[test]
//     fn test_filters_filename_contains() -> Result<()> {
//         let file = PathBuf::from("/home/cabero/Documents/matricula.pdf");
//         let mut filters = Filters::default();
//         filters.filename.contains = "icula".into();
//         if file.matches_filters(&filters) {
//             // the default regex is an empty one, so it should match everything
//             // but we check for this possibility before trying to match
//             Ok(())
//         } else {
//             Err(Error::new(
//                 ErrorKind::Other,
//                 "file did not match the filters correctly".to_string(),
//             ))
//         }
//     }
//     #[test]
//     fn test_filters_filename_endswith() -> Result<()> {
//         let file = PathBuf::from("/home/cabero/Documents/matricula.pdf");
//         let mut filters = Filters::default();
//         filters.filename.contains = "ula.pdf".into();
//         if file.matches_filters(&filters) {
//             // the default regex is an empty one, so it should match everything
//             // but we check for this possibility before trying to match
//             Ok(())
//         } else {
//             Err(Error::new(
//                 ErrorKind::Other,
//                 "file did not match the filters correctly".to_string(),
//             ))
//         }
//     }
// }
