pub mod delete;
pub mod echo;
pub mod io_action;
pub mod script;
pub mod trash;

use crate::{
    path::{Expandable, Update},
    string::Placeholder,
    user_config::rules::{
        actions::{
            delete::Delete,
            echo::Echo,
            io_action::{copy::Copy, r#move::Move, rename::Rename},
            script::Script,
            trash::Trash,
        },
        deserialize::deserialize_path,
    },
};
use log::error;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    fs,
    io::{Error, ErrorKind, Result},
    ops::Deref,
    path::{Path, PathBuf},
    result,
    str::FromStr,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum Action {
    Move(Move),
    Copy(Copy),
    Rename(Rename),
    Delete(Delete),
    Echo(Echo),
    Trash(Trash),
    Script(Script),
}

impl Action {
    fn act<'a>(&self, path: Cow<'a, Path>) -> Result<Cow<'a, Path>> {
        match self {
            Action::Copy(copy) => copy.act(path),
            Action::Delete(delete) => delete.act(path),
            Action::Echo(echo) => echo.act(path),
            Action::Move(r#move) => r#move.act(path),
            Action::Rename(rename) => rename.act(path),
            Action::Trash(trash) => trash.act(path),
            Action::Script(script) => script.act(path),
        }
    }
}

pub trait AsAction {
    fn act<'a>(&self, path: Cow<'a, Path>) -> Result<Cow<'a, Path>>;
    fn kind(&self) -> ActionType;
}

#[derive(Eq, PartialEq)]
pub enum ActionType {
    Copy,
    Delete,
    Echo,
    Move,
    Rename,
    Script,
    Trash,
}

impl ToString for ActionType {
    fn to_string(&self) -> String {
        match self {
            Self::Move => "move",
            Self::Copy => "copy",
            Self::Rename => "rename",
            Self::Delete => "delete",
            Self::Trash => "trash",
            Self::Echo => "echo",
            Self::Script => "script",
        }
        .into()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Actions(Vec<Action>);

impl Deref for Actions {
    type Target = Vec<Action>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Actions {
    pub fn run(&self, path: PathBuf) {
        let mut path = Cow::from(path);
        for action in self.iter() {
            path = match action.act(path) {
                Ok(new_path) => new_path,
                Err(e) => {
                    error!("{}", e);
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::{Error, ErrorKind, Result},
    };

    use crate::{
        path::{
            lib::vars::{expected_path, test_file_or_dir},
            Update,
        },
        user_config::rules::actions::{
            io_action::{ConflictOption, IOAction},
            ActionType,
            ConflictOption,
            IOAction,
        },
    };
    use std::borrow::Cow;

    #[test]
    fn rename_with_rename_conflict() -> Result<()> {
        let original = Cow::from(test_file_or_dir("test2.txt"));
        let expected = expected_path(&original, &Default::default());
        let new_path = original
            .update(&ConflictOption::Rename, &Default::default())
            .unwrap();
        if new_path == expected {
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::Other,
                "filepath after rename is not as expected",
            ))
        }
    }

    #[test]
    fn rename_with_overwrite_conflict() -> Result<()> {
        let original = Cow::from(test_file_or_dir("test2.txt"));
        let expected = original.clone();
        let new_path = original
            .update(&ConflictOption::Overwrite, &Default::default())
            .unwrap();
        if new_path == expected {
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::Other,
                "filepath after rename is not as expected",
            ))
        }
    }

    #[test]
    #[should_panic] // unwrapping a None value
    fn rename_with_skip_conflict() {
        let target = Cow::from(test_file_or_dir("test2.txt"));
        target
            .update(&ConflictOption::Skip, &Default::default())
            .unwrap();
    }

    #[test]
    #[should_panic] // trying to modify a path that does not exist
    fn new_path_to_non_existing_file() {
        let target = Cow::from(test_file_or_dir("test_dir2").join("test1.txt"));
        #[cfg(debug_assertions)]
        debug_assert!(!target.exists());
        target
            .update(&ConflictOption::Rename, &Default::default())
            .unwrap();
    }

    #[test]
    fn prepare_path_copy() -> Result<()> {
        let path = test_file_or_dir("test1.txt");
        let target = test_file_or_dir("test_dir");
        let expected = test_file_or_dir("test_dir").join("test1 (1).txt");
        if expected.exists() {
            fs::remove_file(&expected)?;
        }
        let action = IOAction {
            to: target,
            if_exists: Default::default(),
            sep: Default::default(),
        };
        let new_path = IOAction::helper(&path, &action, ActionType::Copy)?;
        if new_path == expected {
            Ok(())
        } else {
            Err(Error::from(ErrorKind::Other))
        }
    }

    #[test]
    fn prepare_path_move() -> Result<()> {
        let path = test_file_or_dir("test1.txt");
        let target = test_file_or_dir("test_dir");
        let expected = test_file_or_dir("test_dir").join("test1 (1).txt");
        if expected.exists() {
            fs::remove_file(&expected)?;
        }
        let action = IOAction {
            to: target,
            if_exists: Default::default(),
            sep: Default::default(),
        };
        let new_path = IOAction::helper(&path, &action, ActionType::Move)?;
        if new_path == expected {
            Ok(())
        } else {
            Err(Error::from(ErrorKind::Other))
        }
    }

    #[test]
    fn prepare_path_rename() -> Result<()> {
        let path = test_file_or_dir("test1.txt");
        let target = test_file_or_dir("test_dir").join("test1.txt");
        let expected = test_file_or_dir("test_dir").join("test1 (1).txt");
        if expected.exists() {
            fs::remove_file(&expected)?;
        }
        let action = IOAction {
            to: target,
            if_exists: Default::default(),
            sep: Default::default(),
        };
        let new_path = IOAction::helper(&path, &action, ActionType::Rename)?;
        if new_path == expected {
            Ok(())
        } else {
            Err(Error::from(ErrorKind::Other))
        }
    }
}
