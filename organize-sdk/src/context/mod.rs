pub mod scope;
pub mod services;
pub mod settings;

use std::sync::Arc;

use services::{
	fs::{connections::Connections, manager::FileSystemManager},
	history::Journal,
	reporter::{ui::UserInterface, Reporter},
	task_manager::TaskManager,
	Blackboard,
};

use crate::{
	context::{
		scope::ExecutionScope,
		services::{fs::resource::Resource, RunServices},
		settings::RunSettings,
	},
	error::Error,
	templates::compiler::TemplateCompiler,
};

/// The top-level context object, composed of the three distinct categories of information.
#[derive(Clone)]
pub struct ExecutionContext {
	pub services: Arc<RunServices>,
	pub scope: ExecutionScope,
	pub settings: Arc<RunSettings>,
}

impl ExecutionContext {
	pub async fn new(command_run_settings: RunSettings, connections: Connections, ui: Arc<dyn UserInterface>) -> Result<Self, Error> {
		let settings_arc = Arc::new(command_run_settings);

		let journal = Arc::new(Journal::new(&settings_arc).await?);
		let reporter = Reporter::new(ui.clone());
		let task_manager = TaskManager::new(ui.clone());
		let template_compiler = TemplateCompiler::new();
		let fs_manager = FileSystemManager::new(connections, &settings_arc)?;
		let services_arc = Arc::new(RunServices {
			blackboard: Blackboard::default(),
			journal,
			fs: fs_manager,
			template_compiler,
			reporter,
			task_manager,
		});

		Ok(ExecutionContext {
			services: services_arc,
			scope: ExecutionScope::Blank,
			settings: settings_arc,
		})
	}

	pub fn with_scope(&self, scope: ExecutionScope) -> ExecutionContext {
		Self {
			services: self.services.clone(),
			scope,
			settings: self.settings.clone(),
		}
	}

	pub fn with_resource(&self, resource: &Arc<Resource>) -> Result<ExecutionContext, Error> {
		let scope = ExecutionScope::new_resource_scope(self.scope.rule()?, resource.clone());
		Ok(self.with_scope(scope))
	}
}
