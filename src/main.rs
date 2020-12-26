use std::{borrow::Cow, path::PathBuf};

use clap::Clap;
use lazy_static::lazy_static;
use log::error;

use organize_core::data::config::Config;

use crate::cmd::{App, Cmd};

lazy_static! {
	pub static ref CONFIG_PATH: PathBuf = Config::path().unwrap_or_else(|e| {
		error!("{:?}", e);
		std::process::exit(0)
	});
	pub static ref CONFIG_PATH_STR: Cow<'static, str> = CONFIG_PATH.to_string_lossy();
}

mod cmd;

fn main() -> anyhow::Result<()> {
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
