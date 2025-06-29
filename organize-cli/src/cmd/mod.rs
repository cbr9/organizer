use crate::cmd::{logs::LogLevel, run::Run};
use async_trait::async_trait;
use clap::{Parser, Subcommand};
use edit::Edit;
use undo::Undo;

mod edit;
mod logs;
mod run;
mod undo;

#[derive(Subcommand)]
enum Command {
	Run(Run),
	Edit(Edit),
	Undo(Undo),
}

#[derive(Parser)]
#[command(about, author, version)]
pub struct App {
	#[command(subcommand)]
	command: Command,
	#[arg(long, value_enum, global = true, default_value_t = LogLevel::Info)]
	pub log_level: LogLevel,
}

#[async_trait]
pub trait Cmd {
	async fn run(self) -> anyhow::Result<()>;
}

#[async_trait]
impl Cmd for App {
	async fn run(self) -> anyhow::Result<()> {
		let _guard = logs::init(self.log_level);
		match self.command {
			Command::Run(cmd) => cmd.run().await,
			Command::Edit(edit) => edit.run().await,
			Command::Undo(undo) => undo.run().await,
		}
	}
}
