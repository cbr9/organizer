use crate::user_config::rules::actions::AsAction;
use crate::user_config::rules::{
    actions::{ActionType, IOAction},
    deserialize::string_or_struct,
};
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
    fn act(&self, path: &mut Cow<Path>) -> Result<()> {
        IOAction::helper(path, self.deref(), ActionType::Move)
    }
}
