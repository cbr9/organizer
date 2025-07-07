pub mod ui;

use crate::context::services::reporter::ui::UserInterface;
use std::sync::Arc;

/// A service for handling simple, stateless, one-off user-facing messages and logging.
#[derive(Clone)]
pub struct Reporter {
	pub ui: Arc<dyn UserInterface>,
}

impl Reporter {
	pub fn new(ui: Arc<dyn UserInterface>) -> Self {
		Self { ui }
	}

	pub fn success(&self, message: &str) {
		self.ui.success(message);
		tracing::info!(user_message = message, "Operation succeeded.");
	}

	pub fn info(&self, message: &str) {
		self.ui.info(message);
		tracing::info!(user_message = message, "Information displayed.");
	}

	pub fn warning(&self, message: &str) {
		self.ui.warning(message);
		tracing::warn!(user_message = message, "Warning issued.");
	}

	pub fn error(&self, message: &str, hint: Option<&str>) {
		self.ui.error(message, hint);
		tracing::error!(user_message = message, ?hint, "Error occurred.");
	}
}
