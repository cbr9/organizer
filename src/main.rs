use std::{borrow::Cow, path::PathBuf};

use lazy_static::lazy_static;
use log::error;

use organize_core::data::config::Config;

use crate::cmd::{App, Cmd};
use clap::Parser;

lazy_static! {
	pub static ref CONFIG_PATH: PathBuf = Config::path()
		.map(|path| {
			let parent = path.parent().expect("invalid config directory");
			if !parent.exists() {
				std::fs::create_dir_all(&parent).expect("could not create config directory");
			}
			if !path.exists() {
				std::fs::File::create(&path).expect("could not create config file");
			}
			path
		})
		.unwrap_or_else(|e| {
			error!("{:?}", e);
			std::process::exit(0)
		});
	pub static ref CONFIG_PATH_STR: Cow<'static, str> = CONFIG_PATH.to_string_lossy();
}

mod cmd;

fn main() {
	let app: App = App::parse();
	if let Err(e) = app.run() {
		error!("{:?}", e);
	}
}
