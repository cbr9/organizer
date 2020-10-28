use crate::user_config::rules::actions::{ActionType, AsAction};
use log::info;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, fs, io::Result, ops::Deref, path::Path};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Delete(bool);

impl Deref for Delete {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsAction for Delete {
    fn act<'a>(&self, path: Cow<'a, Path>) -> Result<Cow<'a, Path>> {
        if self.0 {
            fs::remove_file(&path)?;
            info!("({}) {}", self.kind().to_string(), path.display());
        }
        Ok(path)
    }

    fn kind(&self) -> ActionType {
        ActionType::Delete
    }
}
