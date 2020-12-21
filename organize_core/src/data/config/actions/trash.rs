use std::path::PathBuf;

use colored::Colorize;
use log::{debug, info};
use serde::Deserialize;

use crate::data::config::actions::{ActionType, AsAction};

#[derive(Debug, Clone, Deserialize, Default, Eq, PartialEq)]
pub struct Trash(bool);

impl AsAction for Trash {
	fn act<T: Into<PathBuf>>(&self, path: T, simulate: bool) -> Option<PathBuf> {
		if self.0 {
			let path = path.into();
			if !simulate {
				trash::delete(&path)
					.map(|_| info!("({}) {}", ActionType::Trash.to_string().bold(), path.display()))
					.map_err(|e| debug!("{}", e))
					.ok()?;
			} else {
				info!("(simulate {}) {}", ActionType::Trash.to_string().bold(), path.display());
			}
		}
		None
	}
	fn ty(&self) -> ActionType {
		ActionType::Trash
	}
}
