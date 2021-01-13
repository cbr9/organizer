use std::ops::Deref;
use std::path::PathBuf;

use colored::Colorize;
use log::info;
use serde::Deserialize;

use crate::simulation::Simulation;
use crate::{
	data::config::actions::{ActionType, AsAction},
	string::{deserialize_placeholder_string, Placeholder},
};

use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Deserialize, Default, Eq, PartialEq)]
pub struct Echo(#[serde(deserialize_with = "deserialize_placeholder_string")] String);

impl Deref for Echo {
	type Target = String;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl AsAction for Echo {
	fn act<T: Into<PathBuf>>(&self, path: T) -> Option<PathBuf> {
		let path = path.into();
		info!("({}) {}", self.ty().to_string().bold(), self.as_str().expand_placeholders(&path).ok()?);
		Some(path)
	}

	fn simulate<T: Into<PathBuf>>(&self, path: T, _simulation: &Arc<Mutex<Simulation>>) -> Option<PathBuf> {
		let path = path.into();
		info!(
			"(simulate {}) {}",
			self.ty().to_string().bold(),
			self.as_str().expand_placeholders(&path).ok()?
		);
		Some(path)
	}

	fn ty(&self) -> ActionType {
		ActionType::Echo
	}
}
