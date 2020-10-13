mod lib;

use std::path::{Path, PathBuf};
use crate::configuration::temporary::conflicts::ConflictOption;
use std::io::{Error, ErrorKind, Read, Write};
use crate::file::get_stem_and_extension;
use std::io;
use colored::Colorize;
use crate::configuration::rules::Rule;
use std::collections::HashMap;
use yaml_rust::YamlEmitter;
use crate::PROJECT_NAME;

/// Helper function for the 'rename' and 'move' actions.
/// It computes the appropriate new path for the file wanting to be renamed or moved.
/// In case of a name conflict, it will decide what new path to return based on a resolver parameter
/// to avoid unwanted overwrites.
/// # Args
/// * `from`: path representing the original file's path
/// * `to`: path representing the target path (must represent a file, which may or may not exist)
/// * `conflict_option`: configuration option that helps adapt the new path
/// # Errors
/// This function will return an error in the following case:
/// * The target path exists and `conflict_option` is set to skip
pub fn new_filepath(
    from: &Path,
    to: &Path,
    conflict_option: &ConflictOption,
    watching: bool,
) -> Result<PathBuf, Error> {
    if to.exists() {
        return match conflict_option {
            ConflictOption::Skip => Ok(from.to_path_buf()),
            ConflictOption::Rename => {
                let (stem, extension) = get_stem_and_extension(to)?;
                let new_dir = to.parent().unwrap();
                let mut new_path = to.to_path_buf();

                if new_path.exists() {
                    let mut n = 1;
                    while new_path.exists() {
                        let new_filename = format!("{} ({:?}).{}", stem, n, extension);
                        new_path = new_dir.join(new_filename);
                        n += 1;
                    }
                }
                Ok(new_path)
            }
            ConflictOption::Overwrite => {
                if to.is_file() {
                    Ok(to.to_path_buf())
                } else if to.is_dir() {
                    Ok(to.join(from.file_name().unwrap()))
                } else {
                    panic!("file is neither a file nor a dir?")
                }
            }
            ConflictOption::Ask => {
                if watching {
                    new_filepath(from, to, Default::default(), false)
                } else {
                    let input = resolve_name_conflict(to)?;
                    new_filepath(from, to, &input, watching)
                }
            }
        };
    }
    Ok(to.to_path_buf())
}

pub fn resolve_name_conflict(dst: &Path) -> Result<ConflictOption, Error> {
    print!(
        "A file named {} already exists in the destination.\n [(o)verwrite / (r)ename / (s)kip]: ",
        dst.file_name().unwrap().to_str().unwrap().underline().bold()
    );
    io::stdout().flush().unwrap();

    let mut buf = [0; 1];
    io::stdin().read_exact(&mut buf).unwrap();
    let buf = buf[0];

    if buf == 111 {
        Ok(ConflictOption::Overwrite)
    } else if buf == 114 {
        Ok(ConflictOption::Rename)
    } else if buf == 115 {
        Ok(ConflictOption::Skip)
    } else {
        Err(Error::new(ErrorKind::InvalidInput, "ERROR: invalid option"))
    }
}

/// returns a hashmap where the keys are paths and the values are tuples of rules
/// and indices that indicate the index of the key's corresponding folder in the rule's folders' list
pub fn path2rules(rules: &[Rule]) -> HashMap<&PathBuf, Vec<(&Rule, usize)>> {
    let mut map = HashMap::new();
    for rule in rules.iter() {
        for (i, folder) in rule.folders.iter().enumerate() {
            if !map.contains_key(&folder.path) {
                map.insert(&folder.path, Vec::new());
            }
            map.get_mut(&folder.path).unwrap().push((rule, i));
        }
    }
    map
}

pub fn combine_options<T>(lhs: Option<T>, rhs: Option<T>, default: Option<T>) -> Option<T> {
    if lhs.is_some() && rhs.is_none() {
        lhs
    } else if lhs.is_none() && rhs.is_some() {
        rhs
    } else if lhs.is_none() && rhs.is_none() {
        default
    } else {
        // both are some
        rhs
    }
}

pub fn create_config_file(path: &Path) -> Result<(), Error> {
    // safe unwrap, dir is created at $HOME or $UserProfile%,
    // so it exists and the user must have permissions
    if path.exists() {
        return Err(Error::new(
            ErrorKind::AlreadyExists,
            format!(
                "{} already exists in this directory",
                path.file_name().unwrap().to_str().unwrap()
            ),
        ));
    }
    match path.parent() {
        Some(parent) => {
            if !parent.exists() {
                std::fs::create_dir_all(path.parent().unwrap())?;
            }
            let default_config = load_yaml!("../../examples/example_config.yml");
            let mut output = String::new();
            let mut emitter = YamlEmitter::new(&mut output);
            emitter
                .dump(default_config)
                .expect("ERROR: could not create starter config");
            std::fs::write(path, output)?;
        }
        None => panic!("home directory's parent folder should be defined"),
    }
    Ok(())
}

pub(crate) fn prompt_editor_env_var() -> String {
    let platform = std::env::consts::OS;
    if platform == "linux" || platform == "macos" {
        format!("{} could not find an $EDITOR environment variable or it's not properly set.\nIn your .bashrc (or .zshrc), set 'export EDITOR=$(which <your-favorite-editor-name>) or \
            run {} as 'EDITOR=$(which <your-favorite-editor-name>) {} config'", PROJECT_NAME, PROJECT_NAME, PROJECT_NAME)
    } else if platform == "windows" {
        format!(
            "{} could not find an EDITOR environment variable or it's not properly set",
            PROJECT_NAME
        )
    } else {
        format!("{} platform not supported", platform)
    }
}