pub mod copy;
pub mod r#move;
pub mod rename;

use crate::{
    path::{Expandable, Update},
    string::Placeholder,
    user_config::rules::{actions::ActionType, deserialize::deserialize_path},
};
use serde::{Deserialize, Serialize};
use std::{
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
#[serde(deny_unknown_fields)]
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
    pub(in crate::user_config::rules) fn helper(
        path: &Path,
        action: &IOAction,
        kind: ActionType,
    ) -> Result<PathBuf> {
        // TODO: refactor this mess
        #[cfg(debug_assertions)]
        debug_assert!([ActionType::Move, ActionType::Rename, ActionType::Copy].contains(&kind));

        let mut to: PathBuf = action
            .to
            .to_str()
            .unwrap()
            .expand_placeholders(path)?
            .deref()
            .into();
        if kind == ActionType::Copy || kind == ActionType::Move {
            if !to.exists() {
                fs::create_dir_all(&to)?;
            }
            to.push(
                path.file_name()
                    .ok_or_else(|| Error::new(ErrorKind::Other, "path has no filename"))?,
            );
        }

        if to.exists() {
            match to.update(&action.if_exists, &action.sep) {
                // FIXME: avoid the into_owned() call
                Ok(new_path) => to = new_path.into_owned(),
                Err(e) => return Err(e),
            }
        }
        Ok(to)
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
