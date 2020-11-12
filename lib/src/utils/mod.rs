#[cfg(test)]
pub mod tests {
	use std::{
		env,
		io::{Error, ErrorKind, Result},
		path::PathBuf,
	};

	use crate::PROJECT_NAME;

	pub trait IntoResult {
		fn into_result(self) -> Result<()>;
	}

	impl IntoResult for bool {
		fn into_result(self) -> Result<()> {
			match self {
				true => Ok(()),
				false => Err(Error::from(ErrorKind::Other)),
			}
		}
	}

	pub fn project() -> PathBuf {
		// when 'cargo test' is run, the current directory should be the project directory
		let cwd = env::current_dir().unwrap().parent().unwrap().to_path_buf();
		cwd
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
