use clap::{Parser, Subcommand};
use tracing::Level;

use crate::cmd::{edit::Edit, run::Run};

mod edit;
mod run;

#[derive(Subcommand)]
enum Command {
	Run(Run),
	Edit(Edit),
}

#[derive(Parser)]
#[command(about, author, version)]
pub struct App {
	#[command(subcommand)]
	command: Command,
	#[arg(long, short = 'v')]
	verbose: bool,
}

pub trait Cmd {
	fn run(self) -> anyhow::Result<()>;
}

impl Cmd for App {
	fn run(self) -> anyhow::Result<()> {
		let format = tracing_subscriber::fmt::format().pretty();
		tracing_subscriber::fmt()
			.event_format(format)
			.with_max_level(Level::DEBUG)
			.init();

		match self.command {
			Command::Run(cmd) => cmd.run(),
			Command::Edit(edit) => edit.run(),
		}
	}
}
