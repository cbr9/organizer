use std::{borrow::Cow, io::Result, ops::Deref, path::Path};

use crate::{
    data::config::actions::{ActionType, AsAction},
    string::{deserialize_placeholder_string, Placeholder},
};
use colored::Colorize;
use log::info;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default, Eq, PartialEq)]
pub struct Echo(#[serde(deserialize_with = "deserialize_placeholder_string")] String);

impl Deref for Echo {
	type Target = String;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl AsAction<Self> for Echo {
	fn act<'a>(&self, path: Cow<'a, Path>) -> Result<Cow<'a, Path>> {
		info!("({}) {}", ActionType::Echo.to_string().bold(), self.as_str().expand_placeholders(&path)?);
		Ok(path)
	}
}
