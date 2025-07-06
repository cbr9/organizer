use std::{fmt::Debug, io::Error};

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
	fn create_progress_indicator(&self, title: &str, length: Option<u64>) -> u64;
	fn increment_progress_indicator(&self, id: u64, delta: u64);
	fn update_progress_indicator(&self, id: u64, description: &str);
	fn remove_progress_indicator(&self, id: u64, style: IndicatorStyle, final_message: &str);

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
