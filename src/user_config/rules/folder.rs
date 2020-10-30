use super::deserialize::{deserialize_full_path, string_or_struct};
use crate::path::Expandable;
use serde::{Deserialize, Serialize};
use std::{ops::Deref, path::PathBuf, str::FromStr};

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
#[serde(deny_unknown_fields)]
pub struct Folder {
    #[serde(deserialize_with = "deserialize_full_path")]
    pub path: PathBuf,
    #[serde(default)]
    pub options: Options,
}

impl From<PathBuf> for Folder {
    fn from(path: PathBuf) -> Self {
        Self {
            path: path.expand_user().expand_vars(),
            options: Default::default(),
        }
    }
}

impl FromStr for Folder {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let path = s.parse::<PathBuf>().unwrap();
        Ok(Self::from(path))
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
pub struct WrappedFolder(#[serde(deserialize_with = "string_or_struct")] Folder);

impl Deref for WrappedFolder {
    type Target = Folder;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub type Folders = Vec<WrappedFolder>;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Options {
    /// defines whether or not subdirectories must be scanned
    #[serde(default)]
    pub recursive: bool,
    #[serde(default)]
    pub watch: bool,
    #[serde(default)]
    pub ignore: Vec<PathBuf>,
    #[serde(default)]
    pub hidden_files: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            recursive: false,
            watch: true,
            hidden_files: false,
            ignore: Vec::new(),
        }
    }
}
