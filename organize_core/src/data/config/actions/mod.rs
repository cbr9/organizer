pub(crate) mod delete;
pub(crate) mod echo;
pub(crate) mod io_action;
pub(crate) mod script;
pub(crate) mod trash;

use std::{borrow::Cow, io::Result, ops::Deref, path::Path};

use crate::data::{
	config::actions::{
		delete::Delete,
		echo::Echo,
		io_action::{Copy, IOAction, Move, Rename},
		script::Script,
		trash::Trash,
	},
	options::apply::Apply,
};
use log::error;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
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

pub(crate) trait AsAction<T> {
	fn act<'a>(&self, path: Cow<'a, Path>) -> Result<Cow<'a, Path>>;
}

#[derive(Eq, PartialEq, ToString)]
#[strum(serialize_all = "lowercase")]
pub enum ActionType {
	Copy,
	Delete,
	Echo,
	Move,
	Rename,
	Script,
	Trash,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
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
					.map(|_| path.to_path_buf())
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
					.map(|_| path.to_path_buf())
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::{data::config::actions::io_action::ConflictOption, path::Update, utils::tests::project};

	#[test]
	fn rename_with_rename_conflict() {
		let original = project().join("tests").join("files").join("test2.txt");
		let expected = original.with_file_name("test2 (1).txt");
		let new_path = original.update(&ConflictOption::Rename, &Default::default()).unwrap();
		assert_eq!(new_path, expected)
	}

	#[test]
	fn rename_with_overwrite_conflict() {
		let original = project().join("tests").join("files").join("test2.txt");
		let new_path = original.update(&ConflictOption::Overwrite, &Default::default()).unwrap();
		assert_eq!(new_path, original)
	}

	#[test]
	fn rename_with_skip_conflict() {
		let original = project().join("tests").join("files").join("test2.txt");
		assert!(original.update(&ConflictOption::Skip, &Default::default()).is_err())
	}
}
