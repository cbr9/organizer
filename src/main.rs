use crate::cmd::{logs::Logs, App, Cmd};
use clap::Clap;
use lazy_static::lazy_static;
use lib::config::UserConfig;
use log::error;
use std::{borrow::Cow, path::PathBuf};

lazy_static! {
	pub static ref DEFAULT_CONFIG: PathBuf = UserConfig::default_path();
	pub static ref DEFAULT_CONFIG_STR: Cow<'static, str> = DEFAULT_CONFIG.to_string_lossy();
}

mod cmd;

fn main() -> anyhow::Result<()> {
	Logs::setup()?;

	if cfg!(target_os = "windows") {
		eprintln!("Windows is not supported yet");
		return Ok(());
	}

	let app: App = App::parse();
	match app.run() {
		Ok(_) => {}
		Err(e) => {
			error!("{}", e);
			std::process::exit(0)
		}
	}
	Ok(())
}
