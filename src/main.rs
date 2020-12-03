use crate::cmd::{logs::Logs, App, Cmd};

use clap::Clap;
use lazy_static::lazy_static;
use log::error;
use organize_core::data::config::UserConfig;
use std::{borrow::Cow, path::PathBuf};

lazy_static! {
	pub static ref CONFIG_PATH: PathBuf = UserConfig::path();
	pub static ref CONFIG_PATH_STR: Cow<'static, str> = CONFIG_PATH.to_string_lossy();
}

mod cmd;

fn main() -> anyhow::Result<()> {
	Logs::setup()?;

	let app: App = App::parse();
	match app.run() {
		Ok(_) => {}
		Err(e) => {
			error!("{:?}", e);
			std::process::exit(0)
		}
	}
	Ok(())
}
