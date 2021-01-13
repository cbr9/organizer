use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::utils::UnwrapMut;
use notify::{DebouncedEvent, Error, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};

use std::time::Duration;

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
		self.watcher.unwrap_mut().watch(&path, RecursiveMode::NonRecursive)?;
		self.watcher.unwrap_mut().watch(path.parent().unwrap(), RecursiveMode::NonRecursive)?;
		let files = path.read_dir()?.filter_map(|file| Some(file.ok()?.path()));
		self.files.extend(files);
		self.folders.insert(path);
		Ok(())
	}

	pub fn unwatch_folder<T: AsRef<Path>>(&mut self, folder: T) -> Result<(), notify::Error> {
		let folder = folder.as_ref();
		match self.watcher {
			None => {}
			Some(ref mut watcher) => {
				match watcher.unwatch(folder) {
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
			}
		}
		Ok(())
	}

	pub fn insert_file<T: Into<PathBuf>>(&mut self, file: T) -> bool {
		self.files.insert(file.into())
	}

	pub fn remove_file<T: AsRef<Path>>(&mut self, file: T) -> bool {
		self.files.remove(file.as_ref())
	}

	fn sync(mut self) -> anyhow::Result<Arc<Mutex<Self>>> {
		use DebouncedEvent::*;

		let (sender, receiver) = channel();
		self.watcher = Some(notify::watcher(sender, Duration::from_secs(0))?);
		let ptr = Arc::new(Mutex::new(self));
		let sim = Arc::clone(&ptr);

		std::thread::spawn(move || {
			while let Ok(event) = receiver.recv() {
				// when the object drops, INotifyWatcher::drop() will be called and this thread will terminate
				match event {
					Remove(path) => {
						if path.is_file() {
							let mut guard = sim.try_lock().unwrap();
							guard.remove_file(path);
						}
					}
					Create(path) => {
						if path.is_file() {
							let mut guard = sim.try_lock().unwrap();
							guard.insert_file(path);
						}
					}
					Rename(from, to) => {
						let mut guard = sim.try_lock().unwrap();
						if guard.folders.contains(&from) && to.is_dir() {
							guard.unwatch_folder(&from).unwrap();
							guard.watch_folder(&to).unwrap();
							continue;
						}
						if guard.files.contains(&from) && to.is_file() {
							guard.remove_file(from);
							guard.insert_file(to);
						}
					}
					_ => {}
				};
			}
		});
		Ok(ptr)
	}
}
