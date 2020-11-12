#[cfg(test)]
pub mod tests {
	use std::{env, path::PathBuf};

	pub fn project() -> PathBuf {
		env::current_dir().unwrap().parent().unwrap().to_path_buf()
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
