use crate::Cmd;
use anyhow::Result;
use clap::Clap;
use colored::Colorize;
use fern::colors::{Color, ColoredLevelConfig};
use organize_core::config::UserConfig;
use std::{fs, path::PathBuf};

#[derive(Debug, Clap)]
pub struct Logs {
	#[clap(long)]
	clear: bool,
}

impl Cmd for Logs {
	fn run(self) -> Result<()> {
		if self.clear {
			fs::remove_file(Self::path()).map_err(anyhow::Error::new)
		} else {
			let text = fs::read_to_string(Self::path())?;
			for line in text.lines() {
				println!("{}", line);
			}
			Ok(())
		}
	}
}

impl Logs {
	pub fn path() -> PathBuf {
		UserConfig::default_dir().join("output.log")
	}

	pub(crate) fn setup() -> Result<(), fern::InitError> {
		let colors = ColoredLevelConfig::new()
			.info(Color::BrightGreen)
			.warn(Color::BrightYellow)
			.error(Color::BrightRed);

		fern::Dispatch::new()
			.format(move |out, message, record| {
				out.finish(format_args!(
					"{} {}: {}",
					chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]").to_string().dimmed(),
					colors.color(record.level()),
					message
				))
			})
			.level(log::LevelFilter::Debug)
			.chain(std::io::stdout())
			.chain(fern::log_file(Self::path())?)
			.apply()?;
		Ok(())
	}
}
