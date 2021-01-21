use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::utils::UnwrapMut;
use notify::{Error, Op, RawEvent, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};

pub struct Simulation {
	pub files: HashSet<PathBuf>,
	folders: HashSet<PathBuf>,
	watcher: Option<RecommendedWatcher>,
}

impl Simulation {
	pub fn new() -> anyhow::Result<Arc<Mutex<Self>>> {
		let sim = Self {
			files: HashSet::new(),
			folders: HashSet::new(),
			watcher: None,
		};

		sim.sync()
	}

	pub fn watch_folder<T: Into<PathBuf>>(&mut self, folder: T) -> anyhow::Result<()> {
		debug_assert!(self.watcher.is_some());
		let path = folder.into();
		if !self.folders.contains(&path) {
			self.watcher.unwrap_mut().watch(&path, RecursiveMode::NonRecursive)?;
			let files = path.read_dir()?.filter_map(|file| Some(file.ok()?.path()));
			self.files.extend(files);
			self.folders.insert(path);
		}
		Ok(())
	}

	pub fn unwatch_folder<T: AsRef<Path>>(&mut self, folder: T) -> Result<(), notify::Error> {
		debug_assert!(self.watcher.is_some());
		let folder = folder.as_ref();
		match self.watcher.unwrap_mut().unwatch(folder) {
			Ok(_) => {}
			Err(e) => match &e {
				Error::Generic(_) | Error::Io(_) => return Err(e),
				Error::PathNotFound | Error::WatchNotFound => {}
			},
		}
		let folders = &self.folders;
		self.files.retain(|file| {
			if let Some(parent) = file.parent() {
				!folders.contains(parent)
			} else {
				false
			}
		});
		self.folders.remove(folder);
		Ok(())
	}

	pub fn insert_file<T: Into<PathBuf>>(&mut self, file: T) -> bool {
		self.files.insert(file.into())
	}

	pub fn remove_file<T: AsRef<Path>>(&mut self, file: T) -> bool {
		self.files.remove(file.as_ref())
	}

	fn sync(mut self) -> anyhow::Result<Arc<Mutex<Self>>> {
		let (sender, receiver) = channel();
		self.watcher = Some(notify::raw_watcher(sender)?);
		let ptr = Arc::new(Mutex::new(self));
		let sim = Arc::clone(&ptr);

		std::thread::spawn(move || loop {
			match receiver.try_recv() {
				Ok(RawEvent {
					path: Some(path),
					op: Ok(op),
					..
				}) => match op {
					Op::REMOVE => {
						let mut guard = sim.lock().unwrap();
						if guard.files.contains(&path) {
							guard.remove_file(path);
						}
					}
					Op::CREATE => {
						if path.is_file() {
							let mut guard = sim.lock().unwrap();
							guard.insert_file(path);
						}
					}
					_ => continue,
				},
				_ => continue,
			}
		});
		Ok(ptr)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::utils::tests::AndWait;
	use std::time::Duration;

	#[test]
	fn simulate() {
		let simulation = Simulation::new().unwrap();
		{
			let mut guard = simulation.lock().unwrap();
			guard.watch_folder("/home/cabero").unwrap();
		}
		let file = PathBuf::from("/home/cabero/simulate_test.pdf");
		// this file must be unique across all tests
		// otherwise if it's created or removed by a different test the thread will pick it up and this test will fail
		std::fs::File::create_and_wait(&file).unwrap();
		std::thread::sleep(Duration::from_millis(100));
		// in most cases the parallel thread should process it before the guard in this thread is created
		// but not in all cases
		{
			let guard = simulation.lock().unwrap();
			assert!(guard.files.contains(&file));
		}
		std::fs::File::remove_and_wait(&file).unwrap();
		std::thread::sleep(Duration::from_millis(100));
		// in most cases the parallel thread should process it before the guard in this thread is created
		// but not in all cases
		{
			let guard = simulation.lock().unwrap();
			assert!(!guard.files.contains(&file));
		}
	}
}
