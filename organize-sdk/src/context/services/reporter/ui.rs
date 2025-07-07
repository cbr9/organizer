use std::{fmt::Debug, io::Error, sync::Arc};

/// Represents the visual style for the final state of a progress indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndicatorStyle {
	Success,
	Warning,
	Error,
}

/// An abstract, generic contract for all user-facing interactions.
pub trait UserInterface: Send + Sync {
	// --- Progress Indicator Management ---
	fn new_progress_bar(&self, title: &str, length: Option<u64>) -> Arc<dyn ProgressBarHandle>;

	// --- User Input ---
	fn input(&self, prompt_text: &str) -> Result<String, Error>;
	fn confirm(&self, prompt_text: &str) -> Result<bool, Error>;
	fn select(&self, prompt: &str, items: &[&str]) -> Result<usize, Error>;

	// --- Structured Messaging ---
	fn info(&self, message: &str);
	fn success(&self, message: &str);
	fn warning(&self, message: &str);
	fn error(&self, message: &str, hint: Option<&str>);
}

pub trait ProgressBarHandle: Send + Sync {
	/// Increments the progress bar by `delta`.
	fn increment(&self, delta: u64);
	/// Updates the description message of the progress bar.
	fn set_message(&self, message: String);
	/// Finishes (removes) the progress bar.
	fn finish(&self);
}
