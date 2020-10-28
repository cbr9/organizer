use crate::user_config::rules::{
    actions::{ActionType, AsAction, IOAction},
    deserialize::string_or_struct,
};
use log::info;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, io::Result, ops::Deref, path::Path};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Move(#[serde(deserialize_with = "string_or_struct")] IOAction);

impl Deref for Move {
    type Target = IOAction;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsAction for Move {
    fn act<'a>(&self, path: Cow<'a, Path>) -> Result<Cow<'a, Path>> {
        let to = IOAction::helper(&path, self, ActionType::Move)?;
        std::fs::rename(&path, &to)?;
        info!(
            "({}) {} -> {}",
            self.kind().to_string(),
            path.display(),
            to.display()
        );
        Ok(Cow::Owned(to))
    }

    fn kind(&self) -> ActionType {
        ActionType::Move
    }
}
