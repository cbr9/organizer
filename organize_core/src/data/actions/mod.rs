use std::{
	ops::Deref,
	path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::{
	data::{
		actions::{
			delete::Delete,
			echo::Echo,
			io_action::{Copy, Hardlink, Move, Rename, Symlink},
			script::Script,
		},
		options::apply::Apply,
	},
	simulation::Simulation,
};

#[cfg(feature = "action_trash")]
use crate::data::actions::delete::Trash;
use anyhow::Result;
use std::sync::{Arc, Mutex, MutexGuard};

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
	#[cfg(feature = "action_trash")]
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
			#[cfg(feature = "action_trash")]
			Trash(trash) => trash.act(from, to),
			Script(script) => script.act(from, to),
		}
	}
}
impl Simulate for Action {
	fn simulate<T, U>(&self, from: T, to: Option<U>, guard: MutexGuard<Simulation>) -> Result<Option<PathBuf>>
	where
		Self: Sized,
		T: AsRef<Path> + Into<PathBuf>,
		U: AsRef<Path> + Into<PathBuf>,
	{
		use Action::*;
		match self {
			Move(r#move) => r#move.simulate(from, to, guard),
			Copy(copy) => copy.simulate(from, to, guard),
			Rename(rename) => rename.simulate(from, to, guard),
			Hardlink(hardlink) => hardlink.simulate(from, to, guard),
			Symlink(symlink) => symlink.simulate(from, to, guard),
			Delete(delete) => delete.simulate(from, to, guard),
			Echo(echo) => echo.simulate(from, to, guard),
			#[cfg(feature = "action_trash")]
			Trash(trash) => trash.simulate(from, to, guard),
			Script(script) => script.simulate(from, to, guard),
		}
	}
}

impl AsAction for Action {
	fn process<T: Into<PathBuf> + AsRef<Path>>(&self, path: T, simulation: Option<&Arc<Mutex<Simulation>>>) -> Option<PathBuf> {
		use Action::*;
		match self {
			Move(r#move) => r#move.process(path, simulation),
			Copy(copy) => copy.process(path, simulation),
			Rename(rename) => rename.process(path, simulation),
			Hardlink(hardlink) => hardlink.process(path, simulation),
			Symlink(symlink) => symlink.process(path, simulation),
			Delete(delete) => delete.process(path, simulation),
			Echo(echo) => echo.process(path, simulation),
			#[cfg(feature = "action_trash")]
			Trash(trash) => trash.process(path, simulation),
			Script(script) => script.process(path, simulation),
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
			#[cfg(feature = "action_trash")]
			Trash(trash) => trash.ty(),
			Script(script) => script.ty(),
		}
	}
}

pub(crate) trait AsAction: Act + Simulate {
	fn process<T: Into<PathBuf> + AsRef<Path>>(&self, path: T, simulation: Option<&Arc<Mutex<Simulation>>>) -> Option<PathBuf>
	where
		Self: Sized;
	fn ty(&self) -> ActionType
	where
		Self: Sized;
}

pub trait Simulate {
	fn simulate<T, U>(&self, from: T, to: Option<U>, guard: MutexGuard<Simulation>) -> Result<Option<PathBuf>>
	where
		Self: Sized,
		T: AsRef<Path> + Into<PathBuf>,
		U: AsRef<Path> + Into<PathBuf>;
}
pub trait Act {
	fn act<T, U>(&self, from: T, to: Option<U>) -> Result<Option<PathBuf>>
	where
		Self: Sized,
		T: AsRef<Path> + Into<PathBuf>,
		U: AsRef<Path> + Into<PathBuf>;
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
			#[cfg(feature = "action_trash")]
			Action::Trash(_) => Self::Trash,
			Action::Script(_) => Self::Script,
		}
	}
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
					path = action.process(path, Some(simulation))?;
				}
				Some(path)
			}
			Apply::AllOf(indices) => {
				let mut path = path.into();
				for i in indices {
					let action = &self.0[*i];
					path = action.process(path, Some(simulation))?;
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
					path = action.process(path, None)?;
				}
				Some(path)
			}
			Apply::AllOf(indices) => {
				let mut path = path.into();
				for i in indices {
					let action = self.0.get(*i)?;
					path = action.process(path, None)?;
				}
				Some(path)
			}
			_ => unreachable!("deserializer should not allow variants 'any' or 'any_of' in `apply.actions`"),
		}
	}
}
