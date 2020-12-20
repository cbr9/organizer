use std::{ops::Deref, path::Path};

use crate::{
	data::config::actions::{ActionType, AsAction},
	string::{deserialize_placeholder_string, Placeholder},
};
use colored::Colorize;
use log::info;
use serde::Deserialize;
use std::path::PathBuf;

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
}
