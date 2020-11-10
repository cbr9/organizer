use serde::{Deserialize, Serialize};
use std::{
	fs,
	fs::OpenOptions,
	io::Result,
	path::{Path, PathBuf},
};

use crate::config::UserConfig;
use num_traits::AsPrimitive;
use std::ops::{Deref, DerefMut};
use sysinfo::{Pid, RefreshKind, System, SystemExt};

/// File where watchers are registered with their PID and configuration
#[derive(Default, Deserialize, Serialize)]
pub struct Register(#[serde(skip)] PathBuf, Vec<Section>);

impl Deref for Register {
	type Target = Vec<Section>;

	fn deref(&self) -> &Self::Target {
		&self.1
	}
}
impl DerefMut for Register {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.1
	}
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Section {
	pub path: PathBuf,
	pub pid: Pid,
}

impl Register {
	pub fn new() -> Result<Self> {
		let path = UserConfig::default_dir().join("register.json");
		OpenOptions::new().write(true).create(true).open(&path)?;
		let content = fs::read_to_string(&path)?;
		let mut register = serde_json::from_str::<Self>(&content)?.update()?;
		register.0 = path;
		Ok(register)
	}

	pub fn append<T, P>(mut self, pid: P, path: T) -> Result<Self>
	where
		T: AsRef<Path>,
		P: AsPrimitive<i32>,
	{
		let section = Section {
			path: path.as_ref().to_path_buf(),
			pid: pid.as_(),
		};
		self.push(section);
		fs::write(&self.0, serde_json::to_string(&self.1)?)?;
		Ok(self)
	}

	pub fn update(mut self) -> Result<Self> {
		if !self.is_empty() {
			let sys = System::new_with_specifics(RefreshKind::with_processes(RefreshKind::new()));
			self.1 = self
				.1
				.into_iter()
				.filter(|section| sys.get_process(section.pid).is_some())
				.collect::<Vec<_>>();
			fs::write(&self.0, serde_json::to_string(&self.0)?)?;
			std::mem::drop(sys);
		}
		Ok(self)
	}
}

#[cfg(test)]
mod tests {
	use std::{convert::TryInto, fs, io::Result};

	use sysinfo::{ProcessExt, RefreshKind, Signal, System, SystemExt};

	use crate::{config::UserConfig, register::Register, utils::tests::IntoResult};

	fn stop() {
		let sys = System::new_with_specifics(RefreshKind::with_processes(RefreshKind::new()));
		let register = Register::new().unwrap();
		register.iter().for_each(|section| {
			sys.get_process(section.pid).unwrap().kill(Signal::Kill);
		});
	}

	fn simulate_watch() {
		let pid = 1000000000i32;
		let sys = System::new_with_specifics(RefreshKind::with_processes(RefreshKind::new()));
		assert!(sys.get_process(pid).is_none());
		let path = UserConfig::default_path();
		let mut register = Register::new().unwrap();
		register.append(pid, &path).unwrap();
	}

	#[test]
	fn clear_dead_processes() -> Result<()> {
		stop();
		simulate_watch();
		let register = Register::new().unwrap();
		register.is_empty().into_result()
	}
}
