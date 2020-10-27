use crate::user_config::rules::actions::AsAction;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::{fs, io::Result, ops::Deref, path::Path};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Delete(bool);

impl Deref for Delete {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsAction for Delete {
    fn act(&self, path: &mut Cow<Path>) -> Result<()> {
        if self.0 {
            return fs::remove_file(path);
        }
        Ok(())
    }
}
