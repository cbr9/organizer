use std::{
	ops::Deref,
	path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::data::{
	actions::{
		delete::Delete,
		echo::Echo,
		io_action::{Copy, Hardlink, Move, Rename, Symlink},
		script::Script,
	},
	options::apply::Apply,
};

use crate::data::actions::delete::Trash;
use anyhow::Result;

pub(crate) mod delete;
pub(crate) mod echo;
pub(crate) mod io_action;
pub(crate) mod script;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all(deserialize = "lowercase"))]
pub enum Action {
	Move(Move),
	Copy(Copy),
	Rename(Rename),
	Hardlink(Hardlink),
	Symlink(Symlink),
	Delete(Delete),
	Echo(Echo),
	Trash(Trash),
	Script(Script),
}

impl Act for Action {
	fn act<T, U>(&self, from: T, to: Option<U>) -> Result<Option<PathBuf>>
	where
		Self: Sized,
		T: AsRef<Path> + Into<PathBuf>,
		U: AsRef<Path> + Into<PathBuf>,
	{
		use Action::*;
		match self {
			Copy(copy) => copy.act(from, to),     // IOAction has three different implementations of AsAction
			Move(r#move) => r#move.act(from, to), // so they must be called with turbo-fish syntax
			Rename(rename) => rename.act(from, to),
			Hardlink(hardlink) => hardlink.act(from, to),
			Symlink(symlink) => symlink.act(from, to),
			Delete(delete) => delete.act(from, to),
			Echo(echo) => echo.act(from, to),
			Trash(trash) => trash.act(from, to),
			Script(script) => script.act(from, to),
		}
	}
}

impl AsAction for Action {
	fn process<T: Into<PathBuf> + AsRef<Path>>(&self, path: T) -> Option<PathBuf> {
		use Action::*;
		match self {
			Move(r#move) => r#move.process(path),
			Copy(copy) => copy.process(path),
			Rename(rename) => rename.process(path),
			Hardlink(hardlink) => hardlink.process(path),
			Symlink(symlink) => symlink.process(path),
			Delete(delete) => delete.process(path),
			Echo(echo) => echo.process(path),
			Trash(trash) => trash.process(path),
			Script(script) => script.process(path),
		}
	}

	fn ty(&self) -> ActionType {
		use Action::*;
		match self {
			Copy(copy) => copy.ty(),
			Move(r#move) => r#move.ty(),
			Rename(rename) => rename.ty(),
			Hardlink(hardlink) => hardlink.ty(),
			Symlink(symlink) => symlink.ty(),
			Delete(delete) => delete.ty(),
			Echo(echo) => echo.ty(),
			Trash(trash) => trash.ty(),
			Script(script) => script.ty(),
		}
	}
}

pub(crate) trait AsAction: Act {
	fn process<T: Into<PathBuf> + AsRef<Path>>(&self, path: T) -> Option<PathBuf>
	where
		Self: Sized;
	fn ty(&self) -> ActionType
	where
		Self: Sized;
}

pub trait Act {
	fn act<T, U>(&self, from: T, to: Option<U>) -> Result<Option<PathBuf>>
	where
		Self: Sized,
		T: AsRef<Path> + Into<PathBuf>,
		U: AsRef<Path> + Into<PathBuf>;
}

#[derive(Eq, PartialEq, Display, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum ActionType {
	Copy,
	Delete,
	Echo,
	Move,
	Rename,
	Hardlink,
	Symlink,
	Script,
	Trash,
}

impl From<&Action> for ActionType {
	fn from(action: &Action) -> Self {
		match action {
			Action::Move(_) => Self::Move,
			Action::Copy(_) => Self::Copy,
			Action::Rename(_) => Self::Rename,
			Action::Hardlink(_) => Self::Hardlink,
			Action::Symlink(_) => Self::Symlink,
			Action::Delete(_) => Self::Delete,
			Action::Echo(_) => Self::Echo,
			Action::Trash(_) => Self::Trash,
			Action::Script(_) => Self::Script,
		}
	}
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Actions(pub Vec<Action>);

impl Deref for Actions {
	type Target = Vec<Action>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl Actions {
	pub fn act<T: Into<PathBuf>>(&self, path: T, apply: &Apply) -> Option<PathBuf> {
		match apply {
			Apply::All => {
				let mut path = path.into();
				for action in self.iter() {
					path = action.process(path)?;
				}
				Some(path)
			}
			Apply::AllOf(indices) => {
				let mut path = path.into();
				for i in indices {
					let action = self.0.get(*i)?;
					path = action.process(path)?;
				}
				Some(path)
			}
			_ => unreachable!("deserializer should not allow variants 'any' or 'any_of' in `apply.actions`"),
		}
	}
}
