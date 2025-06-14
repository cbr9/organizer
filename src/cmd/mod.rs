use crate::cmd::{edit::Edit, run::Run};
use clap::{Parser, Subcommand};

mod edit;
mod logs;
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
}

pub trait Cmd {
	fn run(self) -> anyhow::Result<()>;
}

impl Cmd for App {
	fn run(self) -> anyhow::Result<()> {
		match self.command {
			Command::Run(cmd) => cmd.run(),
			Command::Edit(edit) => edit.run(),
		}
	}
}
