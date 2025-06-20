use async_trait::async_trait;
use std::fmt::Debug;

use anyhow::Result;
use dyn_clone::DynClone;
use dyn_eq::DynEq;

use crate::{config::context::ExecutionContext, errors::Error, resource::Resource, templates::template::Template, utils::backup::Backup};

pub mod common;
// pub mod copy;
// pub mod delete;
// pub mod echo;
// pub mod email;
// pub mod extract;
// pub mod hardlink;
pub mod r#move;
// pub mod script;
// pub mod symlink;
// pub mod trash;
// pub mod write;

#[derive(Default)]
pub enum ExecutionModel {
	#[default]
	Single,
	Batch,
}

#[derive(Debug, Clone, Default)]
pub struct Contract {
	pub created: Vec<Resource>,
	pub deleted: Vec<Resource>,
	pub current: Vec<Resource>,
	pub undo: Vec<Box<dyn Undo>>,
}

dyn_clone::clone_trait_object!(Undo);
dyn_eq::eq_trait_object!(Undo);

#[typetag::serde(tag = "type")]
pub trait Undo: Debug + DynEq + DynClone + Send + Sync {
	/// Executes the reverse operation.
	fn undo(&self) -> Result<()>;

	fn backup(&self) -> Option<&Backup> {
		None
	}
}

dyn_clone::clone_trait_object!(Action);
dyn_eq::eq_trait_object!(Action);

#[async_trait]
#[typetag::serde(tag = "type")]
pub trait Action: DynEq + DynClone + Sync + Send + Debug {
	fn execution_model(&self) -> ExecutionModel {
		ExecutionModel::default()
	}
	async fn execute(&self, _ctx: &ExecutionContext<'_>) -> Result<Contract, Error>;
	fn templates(&self) -> Vec<&Template>;
}
