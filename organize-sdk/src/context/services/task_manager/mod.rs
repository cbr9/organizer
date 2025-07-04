use std::{
	future::Future,
	sync::{
		atomic::{AtomicU64, Ordering},
		Arc,
	},
};

use crate::{context::services::reporter::ui::IndicatorStyle, error::Error};

use super::reporter::ui::UserInterface;

/// Manages the lifecycle of complex, multi-step operations with UI feedback.
/// Its primary public method is `with_task`.
#[derive(Clone)]
pub struct TaskManager {
	ui: Arc<dyn UserInterface>,
}

/// A short-lived handle for a specific running task, passed to the `with_task` closure.
/// It provides the methods for progressing the task.
pub struct TaskReporter {
	task_manager: TaskManager,
	id: u64,
	total_steps: u16,
	current_step: AtomicU64,
}

impl TaskReporter {
	/// Manages the execution of a single step within the task's scope.
	/// This method is consistently async and always expects a Future.
	pub async fn new_step<T, E, F>(self: Arc<Self>, description: &str, operation: F) -> Result<T, Error>
	where
		F: Future<Output = Result<T, E>> + Send,
		T: Send + 'static,
		E: Into<Error> + Send + 'static,
	{
		let step_num = self.current_step.fetch_add(1, Ordering::SeqCst);
		self.task_manager
			.execute_step_internal(self.id, step_num, self.total_steps, description, operation)
			.await
	}
}

impl TaskManager {
	/// Creates a new TaskManager.
	pub fn new(ui: Arc<dyn UserInterface>) -> Self {
		Self { ui }
	}

	/// The primary public method. It manages the full lifecycle of a multi-step task
	/// within a scoped, asynchronous closure.
	pub async fn with_task<F, Fut>(&self, title: &str, total_steps: u16, operation: F) -> Result<(), Error>
	where
		F: FnOnce(Arc<TaskReporter>) -> Fut,
		Fut: Future<Output = Result<(String, IndicatorStyle), Error>>,
	{
		// 1. Setup: TaskManager creates the UI indicator.
		let id = self.ui.create_progress_indicator(title);
		tracing::debug!(id, title, "Task started.");

		let task = Arc::new(TaskReporter {
			task_manager: self.clone(),
			id,
			total_steps,
			current_step: AtomicU64::new(1),
		});

		// 2. Execution: Run the user-provided closure.
		match operation(task).await {
			Ok((success_message, style)) => {
				self.ui.remove_progress_indicator(id, style, &success_message);
				tracing::info!(id, message = success_message, "Task finished successfully.");
				Ok(())
			}
			Err(e) => {
				// 3b. Teardown (Failure): `execute_step_internal` has already
				// handled the UI and logging. We just propagate the error.
				Err(e)
			}
		}
	}

	/// Internal helper that contains the logic for executing a single step.
	async fn execute_step_internal<T, E, F>(&self, id: u64, current_step: u64, total_steps: u16, description: &str, operation: F) -> Result<T, Error>
	where
		F: Future<Output = Result<T, E>> + Send,
		T: Send + 'static,
		E: Into<Error> + Send + 'static,
	{
		// The TaskManager is responsible for formatting the step string.
		let message = format!("[{}/{}] {}", current_step, total_steps, description);
		self.ui.update_progress_indicator(id, current_step, &message);
		tracing::debug!(task_id = id, step = current_step, description, "Starting task step.");

		match operation.await {
			Ok(value) => {
				tracing::debug!(task_id = id, step = current_step, "Task step succeeded.");
				Ok(value)
			}
			Err(e) => {
				let error: Error = e.into();
				self.ui.remove_progress_indicator(id, IndicatorStyle::Error, &error.to_string());
				tracing::error!(id, error = ?error, "Task failed.");
				Err(error)
			}
		}
	}
}
