use std::{fs, io::Result, ops::Deref, path::Path};

use crate::data::config::actions::{ActionType, AsAction};
use colored::Colorize;
use log::{debug, info};
use serde::Deserialize;

use std::path::PathBuf;

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
				match fs::remove_file(&path) {
					Ok(_) => {
						info!("({}) {}", ActionType::Delete.to_string().bold(), path.display());
					}
					Err(e) => {
						debug!("{}", e)
					}
				}
			} else {
				info!("(simulate {}) {}", ActionType::Delete.to_string().bold(), path.display());
			}
		}
		None
	}
}
