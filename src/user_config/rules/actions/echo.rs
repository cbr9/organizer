use crate::{
    string::{Placeholder, PlaceholderStr},
    user_config::rules::actions::{ActionType, AsAction},
};
use colored::Colorize;
use log::info;
use serde::Deserialize;
use std::{borrow::Cow, io::Result, ops::Deref, path::Path};

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Echo(PlaceholderStr);

impl Deref for Echo {
    type Target = PlaceholderStr;

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
