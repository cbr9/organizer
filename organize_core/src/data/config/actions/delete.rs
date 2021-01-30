use std::ops::Deref;
use std::path::{Path, PathBuf};

use crate::data::config::actions::{Act, ActionType, AsAction, Simulate};
use crate::simulation::Simulation;
use anyhow::{Context, Result};
use log::{error, info};
use serde::Deserialize;
use std::str::FromStr;
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
pub struct Delete(bool);

#[cfg(feature = "action_trash")]
#[derive(Debug, Clone, Deserialize, Default, Eq, PartialEq)]
pub struct Trash(bool);

impl Deref for Delete {
	type Target = bool;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

macro_rules! as_action {
	($id:ty) => {
		impl AsAction for $id {
			fn process<T: Into<PathBuf> + AsRef<Path>>(&self, path: T, simulation: Option<&Arc<Mutex<Simulation>>>) -> Option<PathBuf> {
				let path = path.into();
				let to: Option<T> = None;
				if let Some(simulation) = simulation {
					let mut guard = simulation.lock().unwrap();
					let parent = path.parent()?;
					if parent.exists() {
						if let Err(e) = guard.watch_folder(parent) {
							error!("{:?}", e);
							return None;
						};
					}
				}
				if self.0 {
					match simulation {
						None => match self.act(&path, to) {
							Ok(new_path) => {
								info!("({}) {}", self.ty().to_string(), path.display());
								new_path
							}
							Err(e) => {
								error!("{:?}", e);
								None
							}
						},
						Some(simulation) => {
							let guard = simulation.lock().unwrap();
							match self.simulate(&path, to, guard) {
								Ok(new_path) => {
									info!("(simulate {}) {}", self.ty().to_string(), path.display());
									new_path
								}
								Err(e) => {
									error!("{:?}", e);
									None
								}
							}
						}
					}
				} else {
					Some(path)
				}
			}
			fn ty(&self) -> ActionType {
				let name = stringify!($id).to_lowercase();
				ActionType::from_str(&name).expect(&format!("no variant associated with {}", name))
			}
		}
	};
}

as_action!(Delete);
#[cfg(feature = "action_trash")]
as_action!(Trash);

impl Simulate for Delete {
	fn simulate<T, U>(&self, from: T, _to: Option<U>, mut guard: MutexGuard<Simulation>) -> Result<Option<PathBuf>>
	where
		Self: Sized,
		T: AsRef<Path> + Into<PathBuf>,
		U: AsRef<Path> + Into<PathBuf>,
	{
		guard.remove_file(from);
		Ok(None)
	}
}

impl Act for Delete {
	fn act<T, P>(&self, from: T, _to: Option<P>) -> Result<Option<PathBuf>>
	where
		T: AsRef<Path> + Into<PathBuf>,
		P: AsRef<Path> + Into<PathBuf>,
	{
		std::fs::remove_file(&from)
			.map(|_| None)
			.with_context(|| format!("could not delete {}", from.as_ref().display()))
	}
}

#[cfg(feature = "action_trash")]
impl Act for Trash {
	fn act<T, P>(&self, from: T, _to: Option<P>) -> Result<Option<PathBuf>>
	where
		T: AsRef<Path> + Into<PathBuf>,
		P: AsRef<Path> + Into<PathBuf>,
	{
		trash::delete(&from)
			.map(|_| None)
			.with_context(|| format!("could not trash {}", from.as_ref().display()))
	}
}

#[cfg(feature = "action_trash")]
impl Simulate for Trash {
	fn simulate<T, U>(&self, from: T, _to: Option<U>, mut guard: MutexGuard<Simulation>) -> Result<Option<PathBuf>>
	where
		Self: Sized,
		T: AsRef<Path> + Into<PathBuf>,
		U: AsRef<Path> + Into<PathBuf>,
	{
		guard.remove_file(from);
		Ok(None)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::utils::tests::{AndWait, TEST_FILES_DIRECTORY};
	use anyhow::Result;
	use std::fs::File;
	use std::time::Duration;

	#[test]
	fn delete_act_true() -> Result<()> {
		let path = TEST_FILES_DIRECTORY.join("delete_act_true.pdf");
		File::create_and_wait(&path)?;
		let file = Delete(true).process(&path, None);
		std::thread::sleep(Duration::from_millis(100));
		let doesnt_exist = !path.exists();
		let is_none = file.is_none();
		if path.exists() {
			std::fs::remove_file(path)?;
		}
		assert!(doesnt_exist);
		assert!(is_none);
		Ok(())
	}

	#[test]
	fn delete_act_false() -> Result<()> {
		let path = TEST_FILES_DIRECTORY.join("delete_act_false.pdf");
		File::create_and_wait(&path)?;
		let file = Delete(false).process(&path, None);
		let exists = path.exists();
		let is_some = file.is_some();
		std::fs::remove_file(path)?;
		assert!(exists);
		assert!(is_some);
		Ok(())
	}

	#[test]
	fn delete_simulate_false() -> Result<()> {
		let simulation = Simulation::new()?;
		let path = TEST_FILES_DIRECTORY.join("delete_simulate_false.pdf");
		File::create_and_wait(&path)?;
		std::thread::sleep(Duration::from_millis(100));
		let file = Delete(false).process(&path, Some(&simulation));
		{
			let guard = simulation.lock().unwrap();
			let exists = guard.files.contains(&path);
			let is_some = file.is_some();
			std::fs::remove_file(path)?;
			assert!(exists);
			assert!(is_some);
		}
		Ok(())
	}

	#[test]
	fn delete_simulate_true() -> Result<()> {
		let simulation = Simulation::new()?;
		let path = TEST_FILES_DIRECTORY.join("delete_simulate_true.pdf");
		File::create_and_wait(&path)?;
		std::thread::sleep(Duration::from_millis(100));
		let file = Delete(true).process(&path, Some(&simulation));
		{
			let guard = simulation.lock().unwrap();
			let doesnt_exist = !guard.files.contains(&path);
			let is_none = file.is_none();
			std::fs::remove_file(path)?;
			assert!(doesnt_exist);
			assert!(is_none);
		}
		Ok(())
	}

	#[cfg(feature = "action_trash")]
	#[test]
	fn trash_act_true() -> Result<()> {
		let path = TEST_FILES_DIRECTORY.join("trash_act_true.pdf");
		File::create_and_wait(&path)?;
		let file = Trash(true).process(&path, None);
		let is_none = file.is_none(); // for some reason if cwe check that it does not exist on Travis, it fails
		if path.exists() {
			std::fs::remove_file(path)?;
		}
		assert!(is_none);
		Ok(())
	}

	#[cfg(feature = "action_trash")]
	#[test]
	fn trash_act_false() -> Result<()> {
		let path = TEST_FILES_DIRECTORY.join("trash_act_false.pdf");
		File::create_and_wait(&path)?;
		let file = Trash(false).process(&path, None);
		let exists = path.exists();
		let is_some = file.is_some();
		std::fs::remove_file(path)?;
		assert!(exists);
		assert!(is_some);
		Ok(())
	}

	#[cfg(feature = "action_trash")]
	#[test]
	fn trash_simulate_false() -> Result<()> {
		let simulation = Simulation::new()?;
		let path = TEST_FILES_DIRECTORY.join("trash_simulate_false.pdf");
		File::create_and_wait(&path)?;
		std::thread::sleep(Duration::from_millis(100));
		let file = Trash(false).process(&path, Some(&simulation));
		{
			let guard = simulation.lock().unwrap();
			let exists = guard.files.contains(&path);
			let is_some = file.is_some();
			std::fs::remove_file(path)?;
			assert!(exists);
			assert!(is_some);
		}
		Ok(())
	}

	#[cfg(feature = "action_trash")]
	#[test]
	fn trash_simulate_true() -> Result<()> {
		let simulation = Simulation::new()?;
		let path = TEST_FILES_DIRECTORY.join("trash_simulate_true.pdf");
		File::create_and_wait(&path)?;
		std::thread::sleep(Duration::from_millis(100));
		let file = Trash(true).process(&path, Some(&simulation));
		{
			let guard = simulation.lock().unwrap();
			let doesnt_exist = !guard.files.contains(&path);
			let is_none = file.is_none();
			std::fs::remove_file(path)?;
			assert!(doesnt_exist);
			assert!(is_none);
		}
		Ok(())
	}
}
