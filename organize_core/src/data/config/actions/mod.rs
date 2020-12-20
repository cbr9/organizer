pub(crate) mod delete;
pub(crate) mod echo;
pub(crate) mod io_action;
pub(crate) mod script;
pub(crate) mod trash;

use std::{ops::Deref, path::Path};

use crate::data::{
	config::actions::{
		delete::Delete,
		echo::Echo,
		io_action::{Copy, Move, Rename},
		script::Script,
		trash::Trash,
	},
	options::apply::Apply,
};

use serde::Deserialize;

use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all(deserialize = "lowercase"))]
pub enum Action {
	Move(Move),
	Copy(Copy),
	Rename(Rename),
	Delete(Delete),
	Echo(Echo),
	Trash(Trash),
	Script(Script),
}

impl AsAction for Action {
	fn act<T: Into<PathBuf>>(&self, path: T, simulate: bool) -> Option<PathBuf> {
		match self {
			Action::Copy(copy) => copy.act(path, simulate), // IOAction has three different implementations of AsAction
			Action::Move(r#move) => r#move.act(path, simulate), // so they must be called with turbo-fish syntax
			Action::Rename(rename) => rename.act(path, simulate),
			Action::Delete(delete) => delete.act(path, simulate),
			Action::Echo(echo) => echo.act(path, simulate),
			Action::Trash(trash) => trash.act(path, simulate),
			Action::Script(script) => script.act(path, simulate),
		}
	}
}

pub(crate) trait AsAction {
	fn act<P: Into<PathBuf>>(&self, path: P, simulate: bool) -> Option<PathBuf>;
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
pub struct Actions(pub(crate) Vec<Action>);

impl Deref for Actions {
	type Target = Vec<Action>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl Actions {
	pub fn run<T: Into<PathBuf>>(&self, path: T, apply: &Apply, simulate: bool) -> Option<PathBuf> {
		match apply.as_ref() {
			Apply::All => {
				let mut path = path.into();
				for action in self.iter() {
					match action.act(path, simulate) {
						None => return None,
						Some(new_path) => path = new_path,
					}
				}
				Some(path)
			}
			Apply::AllOf(indices) => {
				let mut path = path.into();
				for i in indices {
					match self[*i].act(path, simulate) {
						None => return None,
						Some(new_path) => path = new_path,
					}
				}
				Some(path)
			}
			_ => unreachable!("deserializer should not allow variants 'any' or 'any_of' in `apply.actions`"),
		}
	}
}
