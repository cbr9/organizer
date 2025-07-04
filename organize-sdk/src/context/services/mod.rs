use std::sync::Arc;

use crate::{
	context::services::{fs::manager::FileSystemManager, history::Journal},
	templates::compiler::TemplateCompiler,
};
use dashmap::DashMap;
use reporter::reporter::Reporter;
use std::any::Any;
use task_manager::TaskManager;

pub mod fs;
pub mod history;
pub mod reporter;
pub mod task_manager;

#[derive(Clone)]
pub struct RunServices {
	pub blackboard: Blackboard,
	pub fs: FileSystemManager,
	pub journal: Arc<Journal>,
	pub template_compiler: TemplateCompiler,
	pub reporter: Reporter,
	pub task_manager: TaskManager,
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
