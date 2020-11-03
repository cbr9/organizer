use crate::user_config::{rules::options::Options, UserConfig};
use serde::{Deserialize, Serialize};
use std::{fs, io::Error, path::PathBuf};
use toml::de::Error as TomlError;
use crate::user_config::rules::options::Apply;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Settings {
    #[serde(skip)]
    path: PathBuf,
    pub(crate) defaults: Options,
}

impl AsRef<Self> for Settings {
    fn as_ref(&self) -> &Settings {
        self
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            defaults: Options {
                ignore: Some(Vec::new()),
                hidden_files: Some(false),
                recursive: Some(false),
                watch: Some(true),
                apply: Some(Apply::All)
            },
        }
    }
}

impl Settings {
    pub fn new() -> Result<Self, TomlError> {
        let path = UserConfig::dir().join("settings.toml");
        match fs::read_to_string(&path) {
            Ok(content) => toml::from_str::<Settings>(&content),
            Err(_) => {
                let default = Settings::default();
                let serialized = toml::to_string(&default).unwrap();
                fs::write(&path, serialized).ok();
                Ok(default)
            }
        }
    }
}
