use clap::{Parser, Subcommand};

use crate::cmd::edit::Edit;
use crate::cmd::run::Run;

mod edit;
mod info;
pub(super) mod logs;
mod new;
mod run;
mod stop;
mod watch;

#[derive(Subcommand)]
enum Command {
	Run(Run),
	Edit(Edit),
	// Watch(Watch),
	// Logs(Logs),
	// Stop(Stop),
	// New(New),
	// Info(Info),
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
			// Watch(watch) => watch.run(conn),
			// Run(run) => run.run(conn),
			// Stop(stop) => stop.run(conn),
			// Command::Logs(logs) => {
			// 	Logger::setup(logs.no_color)?;
			// 	logs.run(conn)
			// }
			// Command::New(new) => new.run(conn),
			// Command::Info(info) => info.run(conn),
		}
	}
}
