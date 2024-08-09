use crate::cmd::{App, Cmd};
use anyhow::Result;
use clap::Parser;
mod cmd;

fn main() -> Result<()> {
	let app: App = App::parse();
	app.run()
}
