use clap::{Parser, Subcommand};
use organize_core::logger::Logger;

use crate::cmd::edit::Edit;
use crate::cmd::run::Run;

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
	/// Do not print colored logs
	#[arg(long, default_value_t = false)]
	pub(crate) no_color: bool,
}

pub trait Cmd {
	fn run(self) -> anyhow::Result<()>;
}

impl Cmd for App {
	fn run(self) -> anyhow::Result<()> {
		Logger::setup(self.no_color)?;
		match self.command {
			Command::Run(cmd) => cmd.run(),
			Command::Edit(edit) => edit.run(),
		}
	}
}
