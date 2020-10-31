use crate::{
    string::Placeholder,
    user_config::rules::actions::{ActionType, AsAction},
};
use colored::Colorize;
use log::info;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, io::Result, ops::Deref, path::Path};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Echo(String);

impl Deref for Echo {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsAction<Self> for Echo {
    fn act<'a>(&self, path: Cow<'a, Path>) -> Result<Cow<'a, Path>> {
        info!(
            "({}) {}",
            ActionType::Echo.to_string().bold(),
            self.as_str().expand_placeholders(&path)?
        );
        Ok(path)
    }
}
