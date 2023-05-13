use std::{borrow::Cow, path::PathBuf};

use lazy_static::lazy_static;
use log::error;

use organize_core::data::config::Config;

use crate::cmd::{App, Cmd};
use clap::Parser;
mod cmd;

fn main() {
	let app: App = App::parse();
	if let Err(e) = app.run() {
		error!("{:?}", e);
	}
}
