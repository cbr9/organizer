mod delete;
mod echo;
mod io_action;
mod script;
mod trash;
pub use self::trash::*;
pub use delete::*;
pub use echo::*;
pub use io_action::*;
pub use script::*;

use std::{borrow::Cow, io::Result, ops::Deref, path::Path};

use crate::{config::Apply, file::File};
use log::error;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all(deserialize = "lowercase"))]
pub enum Action {
	Move(IOAction),
	Copy(IOAction),
	Rename(IOAction),
	Delete(Delete),
	Echo(Echo),
	Trash(Trash),
	Script(Script),
}

impl AsAction<Action> for Action {
	fn act<'a>(&self, path: Cow<'a, Path>) -> Result<Cow<'a, Path>> {
		match self {
			Action::Copy(copy) => AsAction::<Copy>::act(copy, path), // IOAction has three different implementations of AsAction
			Action::Move(r#move) => AsAction::<Move>::act(r#move, path), // so they must be called with turbo-fish syntax
			Action::Rename(rename) => AsAction::<Rename>::act(rename, path),
			Action::Delete(delete) => delete.act(path),
			Action::Echo(echo) => echo.act(path),
			Action::Trash(trash) => trash.act(path),
			Action::Script(script) => script.act(path),
		}
	}
}

pub(super) trait AsAction<T> {
	fn act<'a>(&self, path: Cow<'a, Path>) -> Result<Cow<'a, Path>>;
}

#[derive(Eq, PartialEq)]
pub enum ActionType {
	Copy,
	Delete,
	Echo,
	Move,
	Rename,
	Script,
	Trash,
}

impl ToString for ActionType {
	fn to_string(&self) -> String {
		match self {
			Self::Move => "move",
			Self::Copy => "copy",
			Self::Rename => "rename",
			Self::Delete => "delete",
			Self::Trash => "trash",
			Self::Echo => "echo",
			Self::Script => "script",
		}
		.into()
	}
}

#[derive(Debug, Clone, Deserialize)]
pub struct Actions(Vec<Action>);

impl Deref for Actions {
	type Target = Vec<Action>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl Actions {
	pub fn run<A>(&self, path: &Path, apply: A) -> Result<PathBuf>
	where
		A: AsRef<Apply>,
	{
		match apply.as_ref() {
			Apply::Any | Apply::AnyOf(_) => {
				panic!("deserializer should not have allowed variants 'any' or 'any_of' for field 'actions' in option 'apply'")
			}
			Apply::All => {
				let mut path = Cow::from(path);
				self.iter()
					.try_for_each(|action| match action.act(path.clone()) {
						Ok(new_path) => {
							path = new_path;
							Ok(())
						}
						Err(e) => {
							error!("{}", e);
							Err(e)
						}
					})
					.and_then(|_| Ok(path.to_path_buf()))
			}
			Apply::AllOf(indices) => {
				let mut path = Cow::from(path);
				indices
					.iter()
					.try_for_each(|i| match self.get(*i).unwrap().act(path.clone()) {
						Ok(new_path) => {
							path = new_path;
							Ok(())
						}
						Err(e) => {
							error!("{}", e);
							Err(e)
						}
					})
					.and_then(|_| Ok(path.to_path_buf()))
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use std::io::Result;

	use crate::{
		config::ConflictOption,
		path::Update,
		utils::tests::{project, IntoResult},
	};

	#[test]
	fn rename_with_rename_conflict() -> Result<()> {
		let original = project().join("tests").join("files").join("test2.txt");
		let expected = original.with_file_name("test2 (1).txt");
		let new_path = original.update(&ConflictOption::Rename, &Default::default()).unwrap();
		(new_path == expected).into_result()
	}

	#[test]
	fn rename_with_overwrite_conflict() -> Result<()> {
		let original = project().join("tests").join("files").join("test2.txt");
		let new_path = original.update(&ConflictOption::Overwrite, &Default::default()).unwrap();
		(new_path == original).into_result()
	}

	#[test]
	#[should_panic] // unwrapping a None value
	fn rename_with_skip_conflict() {
		let original = project().join("tests").join("files").join("test2.txt");
		original.update(&ConflictOption::Skip, &Default::default()).unwrap();
	}

	#[test]
	#[should_panic] // trying to modify a path that does not exist
	fn new_path_for_non_existing_file() {
		let original = project().join("tests").join("files").join("test_dir2").join("test1.txt");
		debug_assert!(!original.exists());
		original.update(&ConflictOption::Rename, &Default::default()).unwrap();
	}
}
