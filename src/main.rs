use crate::cmd::{App, Cmd};
use clap::Parser;
mod cmd;

fn main() {
	let app: App = App::parse();
	if let Err(e) = app.run() {
		log::error!("{:?}", e);
	}
}
