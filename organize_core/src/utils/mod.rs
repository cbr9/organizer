#[cfg(test)]
pub mod tests {
	use std::{env, path::PathBuf};

	pub fn project() -> PathBuf {
		// when 'cargo test' is run, the current directory should be the project directory
		env::current_dir().unwrap().parent().unwrap().to_path_buf()
	}
}

pub trait UnwrapRef<T> {
	fn unwrap_ref(&self) -> &T;
}

impl<T> UnwrapRef<T> for Option<T> {
	fn unwrap_ref(&self) -> &T {
		self.as_ref().unwrap()
	}
}
