#[cfg(test)]
pub mod tests {
	use crate::{utils::UnwrapRef, PROJECT_NAME};
	use std::{
		borrow::Cow,
		env,
		path::{Path, PathBuf},
	};

	pub fn project() -> PathBuf {
		let mut path = env::current_dir().unwrap();
		while path.file_name().unwrap() != PROJECT_NAME {
			path = path.parent().unwrap().to_path_buf();
		}
		path
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
