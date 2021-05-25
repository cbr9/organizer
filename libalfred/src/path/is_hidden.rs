use std::path::Path;

pub trait IsHidden {
	fn is_hidden(&self) -> bool;
}

impl IsHidden for Path {
	fn is_hidden(&self) -> bool {
		match self.file_name() {
			None => false,
			Some(filename) => filename.to_string_lossy().starts_with('.'),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn check_hidden() {
		let path = Path::new(".testfile");
		assert!(path.is_hidden())
	}
}
