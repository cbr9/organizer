use clap::{Parser, Subcommand};

use crate::cmd::edit::Edit;
use crate::cmd::run::Run;

mod edit;
mod run;
mod watch;

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
}

pub trait Cmd {
	fn run(self) -> anyhow::Result<()>;
}

impl Cmd for App {
	fn run(self) -> anyhow::Result<()> {
		match self.command {
			Command::Run(run) => run.run(),
			Command::Edit(edit) => edit.run(),
		}
	}
}
