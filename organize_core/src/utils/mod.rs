
#[cfg(test)]
pub mod tests {
	use std::path::Path;
	use crate::PROJECT_NAME;
	use std::{env, path::PathBuf};
    use anyhow::Result;
	use std::fs::{File, create_dir_all};
	use std::sync::mpsc::{channel, Receiver};
	use notify::{Watcher, RecursiveMode, Op, RawEvent};
	use std::time::Duration;
	use lazy_static::lazy_static;
	use std::env::temp_dir;

	lazy_static! {
	    pub static ref TEST_FILES_DIRECTORY: PathBuf = {
	    	let dir = temp_dir().join("organize_test");
	    	if !dir.exists() {
	    		create_dir_all(&dir).unwrap();
	    	}
	    	dir
	    };
	}

	pub fn project() -> PathBuf {
		let mut path = env::current_dir().unwrap();
		while path.file_name().unwrap() != PROJECT_NAME {
			path = path.parent().unwrap().to_path_buf();
		}
		path
	}


	pub trait AndWait {
		fn create_and_wait<T: AsRef<Path>>(path: T) -> Result<File>;
		fn remove_and_wait<T: AsRef<Path>>(path: T) -> Result<()>;
		fn wait_for<T: AsRef<Path>>(path: T, event: Op, receiver: Receiver<RawEvent>) -> Result<()>;
	}

	impl AndWait for std::fs::File {
		fn create_and_wait<T: AsRef<Path>>(path: T) -> Result<File> {
			let (sender, receiver) = channel();
			let mut watcher = notify::raw_watcher(sender)?;
			watcher.watch(path.as_ref().parent().unwrap(), RecursiveMode::NonRecursive)?;
    		let file = Self::create(&path)?;
			Self::wait_for(path, Op::CREATE, receiver)?;
			Ok(file)
		}

		fn remove_and_wait<T: AsRef<Path>>(path: T) -> Result<()> {
			let (sender, receiver) = channel();
			let mut watcher = notify::raw_watcher(sender)?;
			watcher.watch(path.as_ref().parent().unwrap(), RecursiveMode::NonRecursive)?;
			std::fs::remove_file(&path)?;
			Self::wait_for(path, Op::REMOVE, receiver)
		}

		fn wait_for<T: AsRef<Path>>(path: T, event: Op, receiver: Receiver<RawEvent>) -> Result<()> {
			let loop_ = || {
				loop {
					if let Ok(RawEvent { path: Some(new_path), op: Ok(op), .. }) = receiver.recv_timeout(Duration::from_secs(2)) {
						if path.as_ref() == new_path && op == event {
							break
						}
					}
				}
			};

			match event {
				Op::CREATE => {
					if !path.as_ref().exists() {
						loop_()
					}
					Ok(())
				},
				Op::REMOVE => {
					if path.as_ref().exists() {
						loop_()
					}
					Ok(())

				},
				_ => unimplemented!()
			}
		}
	}
}

pub trait DefaultOpt {
	fn default_none() -> Self;
	fn default_some() -> Self;
}

pub trait UnwrapOrDefaultOpt<T: DefaultOpt> {
	fn unwrap_or_default_none(self) -> T;
	fn unwrap_or_default_some(self) -> T;
}

impl<T> UnwrapOrDefaultOpt<T> for Option<T>
where
	T: DefaultOpt,
{
	fn unwrap_or_default_none(self) -> T {
		match self {
			None => T::default_none(),
			Some(obj) => obj,
		}
	}

	fn unwrap_or_default_some(self) -> T {
		match self {
			None => T::default_some(),
			Some(obj) => obj,
		}
	}
}

pub trait UnwrapRef<T> {
	fn unwrap_ref(&self) -> &T;
}

pub trait UnwrapMut<T> {
	fn unwrap_mut(&mut self) -> &mut T;
}

impl<T> UnwrapRef<T> for Option<T> {
	fn unwrap_ref(&self) -> &T {
		self.as_ref().unwrap()
	}
}

impl<T> UnwrapMut<T> for Option<T> {
	fn unwrap_mut(&mut self) -> &mut T {
		self.as_mut().unwrap()
	}
}
