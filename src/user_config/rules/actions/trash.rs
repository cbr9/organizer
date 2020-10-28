use crate::user_config::rules::actions::{ActionType, AsAction};
use log::info;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    io::{Error, ErrorKind, Result},
    ops::Deref,
    path::Path,
};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Trash(bool);

impl Deref for Trash {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsAction for Trash {
    fn act<'a>(&self, path: Cow<'a, Path>) -> Result<Cow<'a, Path>> {
        if self.0 {
            return match trash::delete(&path) {
                Ok(_) => {
                    info!("({}) {}", self.kind().to_string(), path.display());
                    Ok(path)
                }
                Err(e) => Err(Error::new(ErrorKind::Other, e.to_string())),
            };
        }
        Ok(path)
    }

    fn kind(&self) -> ActionType {
        ActionType::Trash
    }
}
