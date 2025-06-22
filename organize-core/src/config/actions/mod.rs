use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Debug};

use anyhow::Result;
use dyn_clone::DynClone;
use dyn_eq::DynEq;
use strum_macros::{self, Display};

use crate::{config::context::ExecutionContext, errors::Error, resource::Resource, templates::template::Template, utils::backup::Backup};

pub mod common;
// pub mod copy;
// pub mod delete;
pub mod echo;
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

#[derive(Debug, Serialize, Deserialize, Clone, Display)]
#[strum(serialize_all = "lowercase")]
pub enum Input {
	Processed(Resource),
	Skipped(Resource),
}

#[derive(Debug, Serialize, Deserialize, Clone, Display)]
#[strum(serialize_all = "lowercase")]
pub enum Output {
	Created(Resource),
	Deleted(Resource),
	Modified(Resource),
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Receipt {
	pub inputs: Vec<Input>,
	pub outputs: Vec<Output>,
	pub next: Vec<Resource>,
	pub undo: Vec<Box<dyn Undo>>,
	pub metadata: HashMap<String, serde_json::Value>,
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
	async fn commit(&self, _ctx: &ExecutionContext<'_>) -> Result<Receipt, Error>;
	fn templates(&self) -> Vec<&Template>;
}
