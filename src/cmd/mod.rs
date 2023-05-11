use clap::{Parser, Subcommand};

use self::{run::RunBuilder, watch::WatchBuilder};
use crate::cmd::edit::Edit;

mod edit;
mod run;
mod watch;

#[derive(Subcommand)]
enum Command {
	Run(RunBuilder),
	Edit(Edit),
	Watch(WatchBuilder),
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
			Command::Run(run) => run.build()?.run(),
			Command::Edit(edit) => edit.run(),
			Command::Watch(watch) => watch.build()?.run(),
		}
	}
}
