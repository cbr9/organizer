use crate::{
	cmd::{logs::Logs, App, Cmd},
	user_config::UserConfig,
};
use clap::Clap;
use colored::Colorize;
use fern::colors::{Color, ColoredLevelConfig};
use lazy_static::lazy_static;
use log::error;
use std::{borrow::Cow, path::PathBuf};

pub mod cmd;
pub mod lock_file;
pub mod path;
mod settings;
pub mod string;
pub mod user_config;
pub mod utils;

lazy_static! {
	pub static ref DEFAULT_CONFIG: PathBuf = UserConfig::default_path();
	pub static ref DEFAULT_CONFIG_STR: Cow<'static, str> = DEFAULT_CONFIG.to_string_lossy();
}

fn main() {
	setup_logger().unwrap();

	if cfg!(target_os = "windows") {
		eprintln!("Windows is not supported yet");
		return;
	}

	let app: App = App::parse();
	match app.run() {
		Ok(_) => {}
		Err(e) => {
			error!("{}", e);
			std::process::exit(0)
		}
	}
}

fn setup_logger() -> Result<(), fern::InitError> {
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
		.chain(fern::log_file(Logs::path())?)
		.apply()?;
	Ok(())
}
