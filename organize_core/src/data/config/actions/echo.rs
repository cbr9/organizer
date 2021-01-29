use std::ops::Deref;
use std::path::{Path, PathBuf};

use log::{error, info};
use serde::Deserialize;

use crate::data::config::actions::{Act, Simulate};
use crate::simulation::Simulation;
use crate::{
	data::config::actions::{ActionType, AsAction},
	string::{deserialize_placeholder_string, Placeholder},
};
use anyhow::{Context, Result};
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Debug, Clone, Deserialize, Default, Eq, PartialEq)]
pub struct Echo(#[serde(deserialize_with = "deserialize_placeholder_string")] String);

impl Deref for Echo {
	type Target = String;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl Act for Echo {
	fn act<T, P>(&self, from: T, _to: Option<P>) -> Result<Option<PathBuf>>
	where
		T: AsRef<Path> + Into<PathBuf>,
		P: AsRef<Path> + Into<PathBuf>,
	{
		let from = from.into();
		match self
			.as_str()
			.expand_placeholders(&from)
			.with_context(|| format!("could not expand placeholders ({})", self.as_str()))
		{
			Ok(str) => {
				info!("({}) {}", self.ty().to_string(), str);
				Ok(Some(from))
			}
			Err(e) => {
				error!("{:?}", e);
				Ok(None)
			}
		}
	}
}

impl Simulate for Echo {
	fn simulate<T, U>(&self, from: T, _to: Option<U>, _guard: MutexGuard<Simulation>) -> Result<Option<PathBuf>>
	where
		Self: Sized,
		T: AsRef<Path> + Into<PathBuf>,
		U: AsRef<Path> + Into<PathBuf>,
	{
		let from = from.into();
		match self
			.as_str()
			.expand_placeholders(&from)
			.with_context(|| format!("could not expand placeholders ({})", self.as_str()))
		{
			Ok(str) => {
				info!("(simulate {}) {}", self.ty().to_string(), str);
				Ok(Some(from))
			}
			Err(e) => {
				error!("{:?}", e);
				Ok(None)
			}
		}
	}
}

impl AsAction for Echo {
	fn process<T: Into<PathBuf> + AsRef<Path>>(&self, path: T, simulation: Option<&Arc<Mutex<Simulation>>>) -> Option<PathBuf> {
		let path = path.into();
		let to: Option<T> = None;
		match simulation {
			None => self.act(path, to).unwrap(),
			Some(simulation) => {
				let guard = simulation.lock().unwrap();
				self.simulate(path, to, guard).unwrap()
			}
		}
	}
	fn ty(&self) -> ActionType {
		ActionType::Echo
	}
}
