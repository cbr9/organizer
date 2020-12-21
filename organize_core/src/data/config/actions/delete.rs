use std::{fs, ops::Deref, path::Path};
use std::path::PathBuf;

use colored::Colorize;
use log::{debug, info};
use serde::Deserialize;

use crate::data::config::actions::{ActionType, AsAction};

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
pub struct Delete(bool);

impl Deref for Delete {
	type Target = bool;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl AsAction for Delete {
	fn act<T: Into<PathBuf>>(&self, path: T, simulate: bool) -> Option<PathBuf> {
		let path = path.into();
		if self.0 {
			if !simulate {
				fs::remove_file(&path)
					.map(|_| info!("({}) {}", ActionType::Delete.to_string().bold(), path.display()))
					.map_err(|e| debug!("{}", e))
					.ok()?;
			} else {
				info!("(simulate {}) {}", ActionType::Delete.to_string().bold(), path.display());
			}
		}
		None
	}
	fn ty(&self) -> ActionType {
		ActionType::Delete
	}
}
