pub mod copy;
pub mod delete;
pub mod echo;
pub mod r#move;
pub mod rename;
pub mod script;
pub mod trash;

use crate::{
    path::{Expandable, Update},
    string::Placeholder,
    subcommands::logs::{Level, Logger},
    user_config::rules::{
        actions::{
            copy::Copy, delete::Delete, echo::Echo, r#move::Move, rename::Rename, script::Script,
            trash::Trash,
        },
        deserialize::deserialize_path,
    },
};
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

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct Sep(String);

impl Deref for Sep {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Default for Sep {
    fn default() -> Self {
        Self(" ".into())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq, Default)]
pub struct IOAction {
    #[serde(deserialize_with = "deserialize_path")]
    pub to: PathBuf,
    #[serde(default)]
    pub if_exists: ConflictOption,
    #[serde(default)]
    pub sep: Sep,
}

impl From<PathBuf> for IOAction {
    fn from(path: PathBuf) -> Self {
        Self {
            to: path.expand_user().expand_vars(),
            if_exists: Default::default(),
            sep: Default::default(),
        }
    }
}

impl FromStr for IOAction {
    type Err = ();

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        let path = s.parse::<PathBuf>().unwrap();
        Ok(Self::from(path))
    }
}

impl IOAction {
    /// Helper function for the move, rename and copy actions.
    /// # Args:
    /// - `path`: a reference to a Cow smart pointer containing the original file
    /// - `to`: the destination path of `path`
    /// - `if_exists`: variable that helps resolve any naming conflicts between `path` and `to`
    /// - `sep`: counter separator (e.g. in "file (1).test", `sep` would be a whitespace; in "file-(1).test", it would be a hyphen)
    /// - `type`: whether this helper should move, rename or copy the given file (`path`)
    pub(super) fn helper(path: &mut Cow<Path>, action: &IOAction, kind: ActionType) -> Result<()> {
        // TODO: refactor this mess
        #[cfg(debug_assertions)]
        debug_assert!(
            kind == ActionType::Move || kind == ActionType::Rename || kind == ActionType::Copy
        );

        let mut logger = Logger::default();
        let to = PathBuf::from(
            &action
                .to
                .to_str()
                .unwrap()
                .expand_placeholders(path)
                .unwrap(),
        );
        let mut to = Cow::from(to);
        if kind == ActionType::Copy || kind == ActionType::Move {
            if !to.exists() {
                fs::create_dir_all(&to)?;
            }
            to.to_mut().push(
                path.file_name()
                    .ok_or_else(|| Error::new(ErrorKind::Other, "path has no filename"))?,
            );
        }

        if to.exists() && to.update(&action.if_exists, &action.sep).is_err() {
            return Ok(());
        }

        if kind == ActionType::Copy {
            std::fs::copy(&path, &to)?;
        } else if kind == ActionType::Rename || kind == ActionType::Move {
            std::fs::rename(&path, &to)?;
        }

        logger.try_write(
            &Level::Info,
            &kind,
            &format!("{} -> {}", &path.display(), &to.display()),
        );

        if kind == ActionType::Rename || kind == ActionType::Move {
            *path = to;
        }
        Ok(())
    }
}

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

impl AsAction for Action {
    fn act(&self, path: &mut Cow<Path>) -> Result<()> {
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
    fn act(&self, path: &mut Cow<Path>) -> Result<()>;
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
            action.act(&mut path).ok();
        }
    }
}

/// Defines the options available to resolve a naming conflict,
/// i.e. how the application should proceed when a file exists
/// but it should move/rename/copy some file to that existing path
#[derive(Eq, PartialEq, Debug, Clone, Copy, Deserialize, Serialize)]
// for the config schema to keep these options as lowercase (i.e. the user doesn't have to
// write `if_exists: Rename`), and not need a #[allow(non_camel_case_types)] flag, serde
// provides the option to modify the fields are deserialize/serialize time
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum ConflictOption {
    Overwrite,
    Skip,
    Rename,
    Ask, // not available when watching
}

impl Default for ConflictOption {
    fn default() -> Self {
        ConflictOption::Rename
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Error, ErrorKind, Result};

    use crate::{
        path::{
            lib::vars::{expected_path, test_file_or_dir},
            Update,
        },
        user_config::rules::actions::ConflictOption,
    };
    use std::borrow::Cow;

    #[test]
    fn rename_with_rename_conflict() -> Result<()> {
        let mut target = Cow::from(test_file_or_dir("test2.txt"));
        let expected = expected_path(&target, &Default::default());
        target
            .update(&ConflictOption::Rename, &Default::default())
            .unwrap();
        if target == expected {
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
        let mut target = Cow::from(test_file_or_dir("test2.txt"));
        let expected = target.clone();
        target
            .update(&ConflictOption::Overwrite, &Default::default())
            .unwrap();
        if target == expected {
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
        let mut target = Cow::from(test_file_or_dir("test2.txt"));
        target
            .update(&ConflictOption::Skip, &Default::default())
            .unwrap();
    }

    #[test]
    #[should_panic] // trying to modify a path that does not exist
    fn new_path_to_non_existing_file() {
        let mut target = Cow::from(test_file_or_dir("test_dir2").join("test1.txt"));
        #[cfg(debug_assertions)]
        debug_assert!(!target.exists());
        target
            .update(&ConflictOption::Rename, &Default::default())
            .unwrap();
    }
}
