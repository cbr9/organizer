use std::ops::{Deref, DerefMut};
use std::{path::PathBuf};

use num_traits::AsPrimitive;
use serde::{Deserialize, Serialize};

use sysinfo::{Pid, RefreshKind, System, SystemExt};

use crate::data::Data;

use anyhow::{Context};



mod de;

/// File where watchers are registered with their PID and configuration
#[derive(Default, Serialize)]
pub struct Register {
	#[serde(skip)]
	path: PathBuf,
	#[serde(flatten)]
	sections: Vec<Section>,
}

impl Deref for Register {
	type Target = Vec<Section>;

	fn deref(&self) -> &Self::Target {
		&self.sections
	}
}
impl DerefMut for Register {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.sections
	}
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Section {
	pub path: PathBuf,
	pub pid: Pid,
}

impl Register {
	pub fn new() -> anyhow::Result<Self> {
		let path = Self::path()?;
        if !path.exists() {
            // will be created later
			return Ok(Register { path, ..Register::default() });
		}
		let content = std::fs::read_to_string(&path).context("could not read register")?;
		if content.is_empty() {
			return Ok(Register { path, ..Register::default() });
		}

		let register = match bincode::deserialize::<Self>(content.as_bytes()) {
			Ok(mut register) => {
				register.path = path;
				Ok(register)
			}
			Err(e) => Err(e),
		}?;

		if !register.sections.is_empty() {
			register.update()
		} else {
			Ok(register)
		}
	}

	fn path() -> anyhow::Result<PathBuf> {
		Data::dir().map(|dir| dir.join("register.db"))
	}

	pub fn add<T, P>(mut self, pid: P, path: T) -> anyhow::Result<Self>
	where
		T: Into<PathBuf>,
		P: AsPrimitive<Pid>,
	{
		self.sections.push(Section {
			path: path.into(),
			pid: pid.as_(),
		});
		self.write()?;
		Ok(self)
	}

	fn write(&self) -> anyhow::Result<()> {
		if !self.path.parent().ok_or_else(|| anyhow::Error::msg("invalid data directory"))?.exists() {
			std::fs::create_dir_all(&self.path).context("could not create data directory")?;
		}
        std::fs::write(&self.path, bincode::serialize(&self.sections)?).context("could not write register")
	}

	pub fn update(mut self) -> anyhow::Result<Self> {
		let sys = System::new_with_specifics(RefreshKind::with_processes(RefreshKind::new()));
		self.sections = self
			.sections
			.into_iter()
			.filter(|section| sys.get_process(section.pid).is_some())
			.collect::<Vec<_>>();
		self.write()?;
		Ok(self)
	}
}

#[cfg(test)]
mod tests {
	use sysinfo::{Pid, ProcessExt, RefreshKind, Signal, System, SystemExt};

	use crate::{data::config::Config, register::Register};

	fn stop() {
		let sys = System::new_with_specifics(RefreshKind::with_processes(RefreshKind::new()));
		let register = Register::new().unwrap();
		register.iter().for_each(|section| {
			sys.get_process(section.pid).unwrap().kill(Signal::Kill);
		});
	}

	fn simulate_watch() {
		let pid: Pid = 1000000000;
		let sys = System::new_with_specifics(RefreshKind::with_processes(RefreshKind::new()));
		assert!(sys.get_process(pid).is_none());
		let path = Config::path().unwrap();
		let register = Register::new().unwrap();
		register.add(pid, &path).unwrap();
	}

	#[test]
	fn clear_dead_processes() {
		simulate_watch();
		stop();
		let register = Register::new().unwrap();
		assert!(register.is_empty())
	}
}
