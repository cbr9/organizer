use std::{
	borrow::Cow,
	convert::Infallible,
	ops::Deref,
	path::{Path, PathBuf},
	str::FromStr,
};

use tera::Context;

use crate::{
	config::variables::{AsVariable, Variable},
	templates::CONTEXT,
};

#[derive(Clone)]
pub struct Resource<'a> {
	context: Context,
	variables: &'a [Variable],
	path: PathBuf,
}

impl<'a> FromStr for Resource<'a> {
	type Err = Infallible;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Self {
			context: Context::new(),
			variables: &[],
			path: PathBuf::from_str(s)?,
		})
	}
}

impl<'a> Resource<'a> {
	pub fn new<T: AsRef<Path>>(path: T, variables: &'a [Variable]) -> Self {
		let context = Context::new();
		let mut resource = Self {
			path: path.as_ref().to_path_buf(),
			variables,
			context,
		};
		resource.refresh();
		resource
	}

	fn refresh(&mut self) {
		let path = self.path.to_string_lossy();
		self.context.insert("path", &path);
		for var in self.variables {
			var.register(&mut self.context)
		}
	}

	pub fn context(&mut self) -> Context {
		self.refresh();
		let mut combined_context = Context::new();
		let global = CONTEXT.lock().unwrap().deref().clone();
		combined_context.extend(global);
		combined_context.extend(self.context.clone());
		combined_context
	}

	pub fn path(&mut self) -> Cow<PathBuf> {
		self.refresh();
		Cow::Borrowed(&self.path)
	}

	pub fn set_path<T: AsRef<Path>>(&mut self, path: T) {
		self.path = path.as_ref().into();
		self.refresh();
	}
}
