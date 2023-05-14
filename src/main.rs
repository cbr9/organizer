use log::error;

use crate::cmd::{App, Cmd};
use clap::Parser;
mod cmd;

fn main() {
	let app: App = App::parse();
	if let Err(e) = app.run() {
		error!("{:?}", e);
	}
}
