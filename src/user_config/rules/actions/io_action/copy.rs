use crate::user_config::rules::{
    actions::{io_action::IOAction, ActionType, AsAction},
    deserialize::string_or_struct,
};
use colored::Colorize;
use log::info;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, io::Result, ops::Deref, path::Path};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Copy(#[serde(deserialize_with = "string_or_struct")] IOAction);

impl Deref for Copy {
    type Target = IOAction;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsAction for Copy {
    fn act<'a>(&self, path: Cow<'a, Path>) -> Result<Cow<'a, Path>> {
        let to = IOAction::helper(&path, self, ActionType::Copy)?;
        std::fs::copy(&path, &to)?;
        info!(
            "({}) {} -> {}",
            self.kind().to_string().bold(),
            path.display(),
            to.canonicalize().unwrap().display()
        );
        Ok(path)
    }

    fn kind(&self) -> ActionType {
        ActionType::Copy
    }
}
