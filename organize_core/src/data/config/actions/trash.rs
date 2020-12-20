

use crate::data::config::actions::{ActionType, AsAction};
use colored::Colorize;
use log::{debug, info};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Default, Eq, PartialEq)]
pub struct Trash(bool);

impl AsAction for Trash {
	fn act<T: Into<PathBuf>>(&self, path: T, simulate: bool) -> Option<PathBuf> {
		if self.0 {
			let path = path.into();
			if !simulate {
				match trash::delete(&path) {
					Ok(_) => {
						info!("({}) {}", ActionType::Trash.to_string().bold(), path.display());
					}
					Err(e) => {
						debug!("{}", e);
					}
				}
			} else {
				info!("(simulate {}) {}", ActionType::Trash.to_string().bold(), path.display());
			}
		}
		None
	}
}
