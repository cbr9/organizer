use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};

use num_traits::AsPrimitive;
use serde::{Deserialize, Serialize};

use sysinfo::{Pid, RefreshKind, System, SystemExt};

use crate::data::Data;

use crate::utils::Contains;
use anyhow::{anyhow, Context, Result};

mod de;

/// File where watchers are registered with their PID and configuration
#[derive(Default, Serialize)]
pub struct Register {
	#[serde(skip)]
	path: PathBuf,
	#[serde(flatten)]
	processes: Vec<Process>,
}

impl Deref for Register {
	type Target = Vec<Process>;

	fn deref(&self) -> &Self::Target {
		&self.processes
	}
}
impl DerefMut for Register {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.processes
	}
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Process {
	pub path: PathBuf,
	pub pid: Pid,
}

impl Contains<Pid> for Register {
	fn contains(&self, pid: Pid) -> bool {
		self.processes.iter().any(|section| section.pid == pid)
	}
}

impl Contains<&Path> for Register {
	fn contains(&self, path: &Path) -> bool {
		self.processes.iter().any(|section| section.path == path)
	}
}

impl Register {
	pub fn new() -> Result<Self> {
		let path = Self::path()?;
		if !path.exists() {
			// will be created later
			return Ok(Register { path, ..Register::default() });
		}
		let content = std::fs::read(&path).context("could not read register")?;
		if content.is_empty() {
			return Ok(Register { path, ..Register::default() });
		}

		let mut register = bincode::deserialize::<Self>(content.as_slice())?;
		register.path = path;

		if !register.processes.is_empty() {
			register.update()?;
		}
		Ok(register)
	}

	fn path() -> Result<PathBuf> {
		Data::dir().map(|dir| dir.join("register.db"))
	}

	pub fn push<T, P>(&mut self, pid: P, path: T) -> Result<()>
	where
		T: Into<PathBuf>,
		P: AsPrimitive<Pid>,
	{
		self.processes.push(Process {
			path: path.into(),
			pid: pid.as_(),
		});
		self.persist()
	}

	#[cfg(test)]
	fn clear(&mut self) -> Result<()> {
		self.processes.clear();
		self.persist()
	}

	fn persist(&self) -> Result<()> {
		let parent = self.path.parent().ok_or_else(|| anyhow!("invalid data directory"))?;
		if !parent.exists() {
			std::fs::create_dir_all(&parent).context("could not create data directory")?;
		}
		let bytes = bincode::serialize(&self.processes).context("could not serialize register")?;
		std::fs::write(&self.path, bytes).context("could not write register")
	}

	pub fn update(&mut self) -> Result<()> {
		let sys = System::new_with_specifics(RefreshKind::new().with_processes());
		self.processes.retain(|section| sys.get_process(section.pid).is_some());
		self.persist()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use rand::seq::IteratorRandom;
	use sysinfo::{Pid, ProcessExt, RefreshKind, Signal, System, SystemExt};

	#[test]
	fn clear_dead_processes() -> Result<()> {
		let mut register = Register::new()?;
		let sys = System::new_with_specifics(RefreshKind::with_processes(RefreshKind::new()));
		register.iter().for_each(|section| {
			if let Some(process) = sys.get_process(section.pid) {
				process.kill(Signal::Kill);
			}
		});
		let existing_pid = sys
			.get_processes()
			.keys()
			.choose(&mut rand::thread_rng())
			.copied()
			.unwrap_or_else(rand::random::<Pid>); // in the impossible case that there are no processes running
		let non_existent_pid = loop {
			let pid: Pid = rand::random();
			if sys.get_process(pid).is_none() {
				break pid;
			}
		};
		register.clear()?; // remove all processes from register
		register.push(non_existent_pid, "path1")?;
		register.push(existing_pid, "path2")?;
		register.update()?; // remove non_existent_pid
		assert!(register.contains(existing_pid));
		assert!(!register.contains(non_existent_pid));
		register.clear()?;
		Ok(())
	}
}
