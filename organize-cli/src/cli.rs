use console::{Color, Emoji, style};
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use organize_sdk::context::services::reporter::ui::{IndicatorStyle, UserInterface};
use std::{
	collections::HashMap,
	io::Error,
	sync::{
		Arc,
		Mutex,
		atomic::{AtomicU64, Ordering},
	},
};

/// The CLI-specific implementation of the UserInterface trait.
/// It uses best-in-class crates to create an interactive and
/// user-friendly command-line experience.
pub struct CliUi {
	/// The container that manages the rendering of all active progress bars and spinners.
	multi_progress: MultiProgress,
	/// A thread-safe map to access specific progress indicators by their unique ID.
	bars: Mutex<HashMap<u64, ProgressBar>>,
	/// An atomic counter to generate unique IDs for each progress indicator.
	id_counter: AtomicU64,
}

impl CliUi {
	pub fn new() -> Arc<Self> {
		Arc::new(Self {
			multi_progress: MultiProgress::new(),
			bars: Mutex::new(HashMap::new()),
			id_counter: AtomicU64::new(1),
		})
	}

	fn new_id(&self) -> u64 {
		self.id_counter.fetch_add(1, Ordering::SeqCst)
	}
}

impl UserInterface for CliUi {
	// --- Progress Indicator Management ---

	fn create_progress_indicator(&self, initial_message: &str) -> u64 {
		let pb = self.multi_progress.add(ProgressBar::new_spinner());
		pb.enable_steady_tick(std::time::Duration::from_millis(120));
		// The style is generic; it doesn't know about steps, only a message.
		pb.set_style(
			ProgressStyle::with_template("{spinner:.blue} {wide_msg}")
				.unwrap()
				.tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
		);
		pb.set_message(initial_message.to_string());

		let id = self.new_id();
		self.bars.lock().unwrap().insert(id, pb);
		id
	}

	fn update_progress_indicator(&self, id: u64, step: u64, description: &str) {
		if let Some(pb) = self.bars.lock().unwrap().get(&id) {
			pb.set_position(step);
			pb.set_message(description.to_string());
		}
	}

	fn remove_progress_indicator(&self, id: u64, style: IndicatorStyle, final_message: &str) {
		if let Some(pb) = self.bars.lock().unwrap().remove(&id) {
			let (prefix_emoji, color) = match style {
				IndicatorStyle::Success => (Emoji("✔", "✓"), Color::Green),
				IndicatorStyle::Warning => (Emoji("⚠", "!"), Color::Yellow),
				IndicatorStyle::Error => (Emoji("✖", "X"), Color::Red),
			};

			let styled_message = format!("{} {}", console::style(prefix_emoji).fg(color.into()), final_message);
			pb.finish_with_message(styled_message);
		}
	}

	// --- User Input ---

	fn input(&self, prompt_text: &str) -> Result<String, Error> {
		self.multi_progress.suspend(|| {
			Input::with_theme(&ColorfulTheme::default())
				.with_prompt(prompt_text)
				.interact_text()
		})
	}

	fn confirm(&self, prompt_text: &str) -> Result<bool, Error> {
		self.multi_progress.suspend(|| {
			Confirm::with_theme(&ColorfulTheme::default())
				.with_prompt(prompt_text)
				.interact()
		})
	}

	fn select(&self, prompt: &str, items: &[&str]) -> Result<usize, Error> {
		self.multi_progress.suspend(|| {
			Select::with_theme(&ColorfulTheme::default())
				.with_prompt(prompt)
				.items(items)
				.interact()
		})
	}

	// --- Structured Messaging ---

	fn success(&self, message: &str) {
		let line = format!("{} {}", style(Emoji("✔", "✓")).green(), message);
		self.multi_progress.println(line).unwrap();
	}

	fn info(&self, message: &str) {
		let line = format!("{} {}", style(Emoji("ℹ", "i")).blue(), message);
		self.multi_progress.println(line).unwrap();
	}

	fn warning(&self, message: &str) {
		let line = format!("{} {}", style(Emoji("⚠", "!"),).yellow(), message);
		self.multi_progress.println(line).unwrap();
	}

	fn error(&self, message: &str, hint: Option<&str>) {
		let error_prefix = style("Error:").red().bold();
		let error_line = format!("{} {}", error_prefix, style(message).red());
		self.multi_progress.println(error_line).unwrap();

		if let Some(hint_text) = hint {
			let hint_prefix = style("Hint:").cyan();
			let hint_line = format!("  {} {}", hint_prefix, hint_text);
			self.multi_progress.println(hint_line).unwrap();
		}
	}
}
