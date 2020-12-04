mod de;

use serde::{Deserialize, Serialize};
use std::{
	fs,
	fs::OpenOptions,
	io::Result,
	path::{Path, PathBuf},
};

use num_traits::AsPrimitive;
use serde_json::error::Category;
use std::{
	env::temp_dir,
	ops::{Deref, DerefMut},
};
use sysinfo::{Pid, RefreshKind, System, SystemExt};

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
	pub fn new() -> Result<Self> {
		let path = temp_dir().join("register.json");
		let f = OpenOptions::new().create(true).write(true).read(true).open(&path)?;
		let register = match serde_json::from_reader::<_, Self>(f) {
			Ok(mut register) => {
				register.path = path;
				Ok(register)
			}
			Err(e) => {
				match e.classify() {
					Category::Io | Category::Syntax | Category::Data => Err(e),
					Category::Eof => {
						// the file may be empty
						let register = Register { path, ..Register::default() };
						Ok(register)
					}
				}
			}
		}?;
		Ok(register.update()?)
	}

	pub fn append<T, P>(mut self, pid: P, path: T) -> Result<Self>
	where
		T: AsRef<Path>,
		P: AsPrimitive<Pid>
	{
		let section = Section {
			path: path.as_ref().to_path_buf(),
			pid: pid.as_(),
		};
		self.push(section);
		fs::write(&self.path, serde_json::to_string(&self.sections)?)?;
		Ok(self)
	}

	pub fn update(mut self) -> Result<Self> {
		if !self.is_empty() {
			let sys = System::new_with_specifics(RefreshKind::with_processes(RefreshKind::new()));
			self.sections = self
				.sections
				.into_iter()
				.filter(|section| sys.get_process(section.pid).is_some())
				.collect::<Vec<_>>();
			fs::write(&self.path, serde_json::to_string(&self.sections)?)?;
		}
		Ok(self)
	}
}

#[cfg(test)]
mod tests {

	use sysinfo::{Pid, ProcessExt, RefreshKind, Signal, System, SystemExt};

	use crate::{data::config::UserConfig, register::Register};

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
		let path = UserConfig::path();
		let register = Register::new().unwrap();
		register.append(pid, &path).unwrap();
	}

	#[test]
	fn clear_dead_processes() {
		simulate_watch();
		stop();
		let register = Register::new().unwrap();
		assert!(register.is_empty())
	}
}
