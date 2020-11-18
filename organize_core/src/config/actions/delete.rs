use std::{borrow::Cow, fs, io::Result, ops::Deref, path::Path};

use crate::config::actions::{ActionType, AsAction};
use colored::Colorize;
use log::info;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Delete(bool);

impl Deref for Delete {
	type Target = bool;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl AsAction<Self> for Delete {
	fn act<'a>(&self, path: Cow<'a, Path>) -> Result<Cow<'a, Path>> {
		if self.0 {
			fs::remove_file(&path)?;
			info!("({}) {}", ActionType::Delete.to_string().bold(), path.display());
		}
		Ok(path)
	}
}
