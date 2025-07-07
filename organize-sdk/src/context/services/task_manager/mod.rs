use std::{future::Future, sync::Arc};

use crate::{context::ExecutionContext, error::Error};

use super::reporter::ui::{ProgressBarHandle, UserInterface};

/// Manages the lifecycle of complex, multi-step operations with UI feedback.
/// Its primary public method is `with_task`.
#[derive(Clone)]
pub struct TaskManager {
	ui: Arc<dyn UserInterface>,
}

/// A short-lived handle for a specific running task, passed to the `with_task` closure.
/// It provides the methods for progressing the task.
pub struct ProgressHandle {
	bar: Arc<dyn ProgressBarHandle>,
}

impl std::ops::Deref for ProgressHandle {
	type Target = Arc<dyn ProgressBarHandle>;

	fn deref(&self) -> &Self::Target {
		&self.bar
	}
}

impl ProgressHandle {
	/// Manages the execution of a single step within the task's scope.
	/// This method is consistently async and always expects a Future.
	pub async fn new_long_step<T, E, F>(&self, description: &str, ctx: &ExecutionContext, operation: F) -> Result<T, Error>
	where
		F: Future<Output = Result<T, E>> + Send,
		T: Send + 'static,
		E: Into<Error> + Send + 'static,
	{
		self.bar.set_message(description.to_string());
		tracing::debug!(description, "Starting task step.");

		match operation.await {
			Ok(value) => {
				tracing::debug!("Task step succeeded.");
				Ok(value)
			}
			Err(e) => {
				let error: Error = e.into();
				self.bar.finish();
				ctx.services.reporter.error("Task failed.", None);
				Err(error)
			}
		}
	}
}

impl TaskManager {
	/// Creates a new TaskManager.
	pub fn new(ui: Arc<dyn UserInterface>) -> Self {
		Self { ui }
	}

	/// The primary public method. It manages the full lifecycle of a multi-step task
	/// within a scoped, asynchronous closure.
	pub async fn with_task<F, Fut>(&self, title: &str, length: Option<u64>, ctx: &ExecutionContext, operation: F) -> Result<(), Error>
	where
		F: FnOnce(Arc<ProgressHandle>) -> Fut,
		Fut: Future<Output = Result<String, Error>>,
	{
		// 1. Setup: TaskManager creates the UI indicator.
		let bar = self.ui.new_progress_bar(title, length);
		tracing::debug!(title, "Task started.");

		let task = Arc::new(ProgressHandle { bar: bar.clone() });

		let success_message = operation(task).await?;
		bar.finish();
		ctx.services.reporter.success(&success_message);
		Ok(())
	}
}
