use std::path::PathBuf;
use std::{fs, ops::Deref};

use log::{debug, info};
use serde::Deserialize;

use crate::data::config::actions::{ActionType, AsAction};
use crate::simulation::Simulation;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
pub struct Delete(bool);

impl Deref for Delete {
	type Target = bool;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl AsAction for Delete {
	fn act<T: Into<PathBuf>>(&self, path: T) -> Option<PathBuf> {
		let path = path.into();
		if self.0 {
			fs::remove_file(&path)
				.map(|_| info!("({}) {}", self.ty().to_string(), path.display()))
				.map_err(|e| debug!("{}", e))
				.ok()?;

			None
		} else {
			Some(path)
		}
	}

	fn simulate<T: Into<PathBuf>>(&self, path: T, simulation: &Arc<Mutex<Simulation>>) -> Option<PathBuf> {
		let path = path.into();
		info!("(simulate {}) {}", self.ty().to_string(), path.display());
		let mut ptr = simulation.lock().unwrap();
		ptr.watch_folder(path.parent()?).ok()?;
		ptr.remove_file(path);
		None
	}

	fn ty(&self) -> ActionType {
		ActionType::Delete
	}
}
