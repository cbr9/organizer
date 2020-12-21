use std::ops::Deref;
use std::path::PathBuf;

use colored::Colorize;
use log::info;
use serde::Deserialize;

use crate::{
	data::config::actions::{ActionType, AsAction},
	string::{deserialize_placeholder_string, Placeholder},
};

#[derive(Debug, Clone, Deserialize, Default, Eq, PartialEq)]
pub struct Echo(#[serde(deserialize_with = "deserialize_placeholder_string")] String);

impl Deref for Echo {
	type Target = String;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl AsAction for Echo {
	fn act<T: Into<PathBuf>>(&self, path: T, simulate: bool) -> Option<PathBuf> {
		let path = path.into();
		if !simulate {
			info!(
				"({}) {}",
				ActionType::Echo.to_string().bold(),
				self.as_str().expand_placeholders(&path).ok()?
			);
		} else {
			info!(
				"(simulate {}) {}",
				ActionType::Echo.to_string().bold(),
				self.as_str().expand_placeholders(&path).ok()?
			);
		}
		Some(path)
	}
	fn ty(&self) -> ActionType {
		ActionType::Echo
	}
}
