use std::{
	convert::Infallible,
	hash::Hash,
	path::{Path, PathBuf},
	str::FromStr,
};



#[derive(Debug, Clone)]
pub struct Resource {
	pub path: PathBuf,
	pub root: PathBuf,
}

impl Eq for Resource {}
impl PartialEq for Resource {
	fn eq(&self, other: &Self) -> bool {
		self.path.eq(&other.path)
	}
}

impl Hash for Resource {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.path.hash(state)
	}
}

impl FromStr for Resource {
	type Err = Infallible;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let path = PathBuf::from_str(s)?;
		let parent = path.parent().unwrap().to_path_buf();
		Ok(Self::new(path, parent))
	}
}

impl Resource {
	pub fn new<T: AsRef<Path>, P: AsRef<Path>>(path: T, root: P) -> Self {
		Self {
			path: path.as_ref().to_path_buf(),
			root: root.as_ref().to_path_buf(),
		}
	}

	pub fn set_path<T: AsRef<Path>>(&mut self, path: T) {
		self.path = path.as_ref().into();
	}
}

impl<T: AsRef<Path>> From<T> for Resource {
	fn from(value: T) -> Self {
		Resource::new(value.as_ref(), value.as_ref().parent().unwrap())
	}
}
