use crate::user_config::rules::actions::AsAction;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::{
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
    fn act(&self, path: &mut Cow<Path>) -> Result<()> {
        if self.0 {
            return match trash::delete(path) {
                Ok(_) => Ok(()),
                Err(e) => Err(Error::new(ErrorKind::Other, e.to_string())),
            };
        }
        Ok(())
    }
}
