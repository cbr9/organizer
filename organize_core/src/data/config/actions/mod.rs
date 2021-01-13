use std::ops::Deref;
use std::path::PathBuf;

use serde::Deserialize;

use crate::data::config::actions::io_action::{Hardlink, Symlink};
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
use crate::simulation::Simulation;
use std::sync::{Arc, Mutex};

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
	Hardlink(Hardlink),
	Symlink(Symlink),
	Delete(Delete),
	Echo(Echo),
	Trash(Trash),
	Script(Script),
}

impl AsAction for Action {
	fn act<T: Into<PathBuf>>(&self, path: T) -> Option<PathBuf> {
		match self {
			Action::Copy(copy) => copy.act(path),     // IOAction has three different implementations of AsAction
			Action::Move(r#move) => r#move.act(path), // so they must be called with turbo-fish syntax
			Action::Rename(rename) => rename.act(path),
			Action::Hardlink(hardlink) => hardlink.act(path),
			Action::Symlink(symlink) => symlink.act(path),
			Action::Delete(delete) => delete.act(path),
			Action::Echo(echo) => echo.act(path),
			Action::Trash(trash) => trash.act(path),
			Action::Script(script) => script.act(path),
		}
	}

	fn simulate<T: Into<PathBuf>>(&self, path: T, simulation: &Arc<Mutex<Simulation>>) -> Option<PathBuf> {
		match self {
			Action::Move(r#move) => r#move.simulate(path, simulation),
			Action::Copy(copy) => copy.simulate(path, simulation),
			Action::Rename(rename) => rename.simulate(path, simulation),
			Action::Hardlink(hardlink) => hardlink.simulate(path, simulation),
			Action::Symlink(symlink) => symlink.simulate(path, simulation),
			Action::Delete(delete) => delete.simulate(path, simulation),
			Action::Echo(echo) => echo.simulate(path, simulation),
			Action::Trash(trash) => trash.simulate(path, simulation),
			Action::Script(script) => script.simulate(path, simulation),
		}
	}

	fn ty(&self) -> ActionType {
		match self {
			Action::Copy(copy) => copy.ty(),
			Action::Move(r#move) => r#move.ty(),
			Action::Rename(rename) => rename.ty(),
			Action::Hardlink(hardlink) => hardlink.ty(),
			Action::Symlink(symlink) => symlink.ty(),
			Action::Delete(delete) => delete.ty(),
			Action::Echo(echo) => echo.ty(),
			Action::Trash(trash) => trash.ty(),
			Action::Script(script) => script.ty(),
		}
	}
}

pub(crate) trait AsAction {
	fn act<T: Into<PathBuf>>(&self, path: T) -> Option<PathBuf>;
	fn simulate<T: Into<PathBuf>>(&self, path: T, simulation: &Arc<Mutex<Simulation>>) -> Option<PathBuf>;
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
	Hardlink,
	Symlink,
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
	pub fn simulate<T: Into<PathBuf>>(&self, path: T, apply: &Apply, simulation: &Arc<Mutex<Simulation>>) -> Option<PathBuf> {
		match apply {
			Apply::All => {
				let mut path = path.into();
				for action in self.iter() {
					path = action.simulate(path, simulation)?;
				}
				Some(path)
			}
			Apply::AllOf(indices) => {
				let mut path = path.into();
				for i in indices {
					let action = &self.0[*i];
					path = action.simulate(path, simulation)?;
				}
				Some(path)
			}
			_ => unreachable!("deserializer should not allow variants 'any' or 'any_of' in `apply.actions`"),
		}
	}

	pub fn act<T: Into<PathBuf>>(&self, path: T, apply: &Apply) -> Option<PathBuf> {
		match apply {
			Apply::All => {
				let mut path = path.into();
				for action in self.iter() {
					path = action.act(path)?;
				}
				Some(path)
			}
			Apply::AllOf(indices) => {
				let mut path = path.into();
				for i in indices {
					let action = self.0.get(*i)?;
					path = action.act(path)?;
				}
				Some(path)
			}
			_ => unreachable!("deserializer should not allow variants 'any' or 'any_of' in `apply.actions`"),
		}
	}
}
