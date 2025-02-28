use std::{
	convert::Infallible,
	hash::Hash,
	path::{Path, PathBuf},
	str::FromStr,
};

use tera::Context;

use crate::config::variables::{AsVariable, Variable};

#[derive(Debug, Clone)]
pub struct Resource {
	pub context: Context,
	variables: Vec<Variable>,
	pub path: PathBuf,
	pub last_event_id: Option<i32>,
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
		Ok(Self::new(path, parent, vec![]))
	}
}

impl Resource {
	pub fn new<T: AsRef<Path>, P: AsRef<Path>>(path: T, root: P, variables: Vec<Variable>) -> Self {
		let mut context = Context::new();
		context.insert("root", &root.as_ref().to_string_lossy());
		let mut resource = Self {
			path: path.as_ref().to_path_buf(),
			variables,
			context,
			last_event_id: None,
		};
		resource.refresh();
		resource
	}

	fn refresh(&mut self) {
		let path = self.path.to_string_lossy();
		self.context.insert("path", &path);
		for var in self.variables.iter() {
			var.register(&mut self.context)
		}
	}

	pub fn set_path<T: AsRef<Path>>(&mut self, path: T) {
		self.path = path.as_ref().into();
		self.refresh();
	}
}

impl<T: AsRef<Path>> From<T> for Resource {
	fn from(value: T) -> Self {
		Resource::new(value.as_ref(), value.as_ref().parent().unwrap(), vec![])
	}
}
