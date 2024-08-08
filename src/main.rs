use crate::cmd::{App, Cmd};
use clap::Parser;
use once_cell::sync::OnceCell;
mod cmd;
use organize_core::config::Config;

fn main() {
	let app: App = App::parse();
	if let Err(e) = app.run() {
		log::error!("{:?}", e);
	}
}
