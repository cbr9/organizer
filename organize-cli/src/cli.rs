use console::{Color, Emoji, style};
use dashmap::DashMap;
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use organize_sdk::context::services::reporter::ui::{IndicatorStyle, UserInterface};
use std::{
	io::Error,
	sync::{
		Arc,
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
	bars: DashMap<u64, ProgressBar>,
	/// An atomic counter to generate unique IDs for each progress indicator.
	id_counter: AtomicU64,
}

impl CliUi {
	pub fn new() -> Arc<Self> {
		Arc::new(Self {
			multi_progress: MultiProgress::new(),
			bars: DashMap::new(),
			id_counter: AtomicU64::new(1),
		})
	}

	fn new_id(&self) -> u64 {
		self.id_counter.fetch_add(1, Ordering::SeqCst)
	}
}

impl Drop for CliUi {
	/// This code will run when the `CliUi` struct is dropped at the end of `main`.
	fn drop(&mut self) {
		// `clear()` finishes all progress bars and clears them from the screen,
		// ensuring their final state is visible before the program exits.
		if let Err(e) = self.multi_progress.clear() {
			eprintln!("Error clearing progress bars: {}", e);
		}
	}
}

impl UserInterface for CliUi {
	fn create_progress_indicator(&self, initial_message: &str, length: Option<u64>) -> u64 {
		let pb = match length {
			Some(len) => {
				let bar = self.multi_progress.add(ProgressBar::new(len));
				bar.set_style(
					ProgressStyle::with_template("{wide_msg} {bytes}/{total_bytes} ({eta}) [{bar:40.cyan/blue}]")
						.unwrap()
						.progress_chars("=>-"),
				);
				bar
			}
			None => {
				let spinner = self.multi_progress.add(ProgressBar::new_spinner());
				spinner.set_style(ProgressStyle::with_template("{spinner:.blue} {wide_msg}").unwrap());
				spinner
			}
		};
		pb.set_message(initial_message.to_string());
		let id = self.new_id();
		self.bars.insert(id, pb);
		id
	}

	fn increment_progress_indicator(&self, id: u64, delta: u64) {
		if let Some(pb) = self.bars.get(&id) {
			pb.inc(delta);
		}
	}

	fn update_progress_indicator(&self, id: u64, message: &str) {
		if let Some(pb) = self.bars.get(&id) {
			pb.set_message(message.to_string());
		}
	}

	fn remove_progress_indicator(&self, id: u64, style: IndicatorStyle, final_message: &str) {
		if let Some((_, pb)) = self.bars.remove(&id) {
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
