use crate::cmd::{App, Cmd};
use anyhow::Result;
use clap::Parser;
use organize_core::{config::context::RunSettings, engine::Engine};
mod cmd;

#[tokio::main]
async fn main() -> Result<()> {
	let app: App = App::parse();
	app.run().await
}
