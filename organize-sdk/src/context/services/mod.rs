use std::sync::Arc;

use crate::{
    context::services::{fs::manager::FileSystemManager, history::Journal},
    templates::compiler::TemplateCompiler,
};
use dashmap::DashMap;
use std::any::Any;

pub mod fs;
pub mod history;

#[derive(Debug, Clone)]
pub struct RunServices {
	pub blackboard: Blackboard,
	pub fs: FileSystemManager,
	pub journal: Arc<Journal>,
	pub compiler: TemplateCompiler,
}

#[derive(Debug, Clone)]
pub struct Blackboard {
	pub scratchpad: Arc<DashMap<String, Box<dyn Any + Send + Sync>>>,
	pub shared_context: Arc<DashMap<String, String>>,
}

impl Default for Blackboard {
	fn default() -> Self {
		Self {
			scratchpad: Arc::new(DashMap::new()),
			shared_context: Arc::new(DashMap::new()),
		}
	}
}