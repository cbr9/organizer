use console::{Emoji, style};
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};
use indicatif::{MultiProgress, ProgressStyle};
use organize_sdk::context::services::reporter::ui::{ProgressBarHandle, UserInterface};
use std::{io::Error, sync::Arc};

/// The CLI-specific implementation of the UserInterface trait.
/// It uses best-in-class crates to create an interactive and
/// user-friendly command-line experience.
pub struct CliUi {
	/// The container that manages the rendering of all active progress bars and spinners.
	multi_progress: MultiProgress,
}

pub struct ProgressBar(indicatif::ProgressBar);

impl ProgressBar {
	pub fn new(pb: indicatif::ProgressBar) -> Self {
		Self(pb)
	}
}

impl ProgressBarHandle for ProgressBar {
	fn increment(&self, delta: u64) {
		self.0.inc(delta);
	}

	fn set_message(&self, message: String) {
		self.0.set_message(message);
	}

	fn finish(&self) {
		self.0.finish_and_clear();
	}
}

impl CliUi {
	pub fn new() -> Arc<Self> {
		Arc::new(Self {
			multi_progress: MultiProgress::new(),
		})
	}
}

impl UserInterface for CliUi {
	fn new_progress_bar(&self, initial_message: &str, length: Option<u64>) -> Arc<dyn ProgressBarHandle> {
		let pb = match length {
			Some(len) => {
				let bar = self.multi_progress.add(indicatif::ProgressBar::new(len));
				bar.set_style(
					ProgressStyle::with_template("{wide_msg} {bytes}/{total_bytes} ({eta}) [{bar:40.cyan/blue}]")
						.unwrap()
						.progress_chars("=>-"),
				);
				bar
			}
			None => {
				let spinner = self.multi_progress.add(indicatif::ProgressBar::new_spinner());
				spinner.set_style(ProgressStyle::with_template("{spinner:.blue} {wide_msg}").unwrap());
				spinner
			}
		};
		pb.set_message(initial_message.to_string());
		Arc::new(ProgressBar::new(pb))
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
		let line = format!("{}  {}", style(Emoji("✔", "✓")).green(), message);
		self.multi_progress.println(line).unwrap();
	}

	fn info(&self, message: &str) {
		let line = format!("{}  {}", style(Emoji("ℹ", "i")).blue(), message);
		self.multi_progress.println(line).unwrap();
	}

	fn warning(&self, message: &str) {
		let line = format!("{}  {}", style(Emoji("⚠", "!"),).yellow(), message);
		self.multi_progress.println(line).unwrap();
	}

	fn error(&self, message: &str, hint: Option<&str>) {
		let error_prefix = style("Error:").red().bold();
		let error_line = format!("{}  {}", error_prefix, style(message).red());
		self.multi_progress.println(error_line).unwrap();

		if let Some(hint_text) = hint {
			let hint_prefix = style("Hint:").cyan();
			let hint_line = format!("  {} {}", hint_prefix, hint_text);
			self.multi_progress.println(hint_line).unwrap();
		}
	}
}
