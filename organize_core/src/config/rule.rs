use std::{borrow::BorrowMut, collections::HashSet, path::Path};

use anyhow::Result;
use serde::Deserialize;
use tera::{Context, Tera};

use crate::{
	templates::{CONTEXT, TERA},
	utils::DefaultOpt,
};

use super::{
	actions::{Action, ActionPipeline},
	filters::Filters,
	folders::Folders,
	options::FolderOptions,
};

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Rule {
	pub id: Option<String>,
	#[serde(default)]
	pub tags: HashSet<String>,
	#[serde(default)]
	pub r#continue: bool,
	pub actions: Vec<Action>,
	pub filters: Filters,
	pub folders: Folders,
	#[serde(default = "FolderOptions::default_none")]
	pub options: FolderOptions,
	#[serde(default)]
	pub variables: Vec<Variable>,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all(deserialize = "lowercase"))]
pub enum Variable {
	Simple(SimpleVariable),
}

impl AsVariable for Variable {
	fn register(&self) {
		match self {
			Variable::Simple(s) => s.register(),
		}
	}
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct SimpleVariable {
	name: String,
	value: String,
}

pub trait AsVariable {
	fn register(&self);
}

impl AsVariable for SimpleVariable {
	fn register(&self) {
		let mut ctx = CONTEXT.lock().unwrap();
		let value = TERA.lock().unwrap().render_str(&self.value, ctx.borrow_mut()).unwrap();
		ctx.insert(&self.name, &value);
	}
}

impl Default for Rule {
	fn default() -> Self {
		Self {
			id: None,
			tags: HashSet::new(),
			r#continue: false,
			variables: vec![],
			actions: vec![],
			filters: Filters(vec![]),
			folders: vec![],
			options: FolderOptions::default_none(),
		}
	}
}
