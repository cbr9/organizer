use std::{ops::Deref};
use std::path::PathBuf;

use serde::Deserialize;

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

pub(crate) mod delete;
pub(crate) mod echo;
pub(crate) mod io_action;
pub(crate) mod script;
pub(crate) mod trash;

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
	fn ty(&self) -> ActionType {
		match self {
			Action::Copy(copy) => copy.ty(),
			Action::Move(r#move) => r#move.ty(), // so they must be called with turbo-fish syntax
			Action::Rename(rename) => rename.ty(),
			Action::Delete(delete) => delete.ty(),
			Action::Echo(echo) => echo.ty(),
			Action::Trash(trash) => trash.ty(),
			Action::Script(script) => script.ty(),
		}
	}
}

pub(crate) trait AsAction {
	fn act<P: Into<PathBuf>>(&self, path: P, simulate: bool) -> Option<PathBuf>;
	fn ty(&self) -> ActionType;
}

#[derive(Eq, PartialEq, ToString, EnumString)]
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
					path = action.act(path, simulate)?;
				}
				Some(path)
			}
			Apply::AllOf(indices) => {
				let mut path = path.into();
				for i in indices {
					let action = self.0.get(*i)?;
					path = action.act(path, simulate)?;
				}
				Some(path)
			}
			_ => unreachable!("deserializer should not allow variants 'any' or 'any_of' in `apply.actions`"),
		}
	}
}
