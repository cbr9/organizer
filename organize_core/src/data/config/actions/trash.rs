use std::path::PathBuf;

use colored::Colorize;
use log::{debug, info};
use serde::Deserialize;

use crate::data::config::actions::{ActionType, AsAction};
use crate::simulation::Simulation;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Deserialize, Default, Eq, PartialEq)]
pub struct Trash(bool);

impl AsAction for Trash {
	fn act<T: Into<PathBuf>>(&self, path: T) -> Option<PathBuf> {
		let path = path.into();
		if self.0 {
			trash::delete(&path)
				.map(|_| info!("({}) {}", ActionType::Trash.to_string().bold(), path.display()))
				.map_err(|e| debug!("{}", e))
				.ok()?;
			None
		} else {
			Some(path)
		}
	}

	fn simulate<T: Into<PathBuf>>(&self, path: T, simulation: &Arc<Mutex<Simulation>>) -> Option<PathBuf> {
		let path = path.into();
		info!("(simulate {}) {}", self.ty().to_string().bold(), path.display());
		let mut ptr = simulation.lock().unwrap();
		ptr.watch_folder(path.parent()?).ok()?;
		ptr.remove_file(path);
		None
	}

	fn ty(&self) -> ActionType {
		ActionType::Trash
	}
}
