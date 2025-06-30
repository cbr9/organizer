use crate::cmd::{logs::LogLevel, run::Run};
use async_trait::async_trait;
use clap::{Parser, Subcommand};
use undo::Undo;

mod logs;
mod run;
mod undo;

#[derive(Subcommand)]
enum Command {
	Run(Run),
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
			Command::Undo(undo) => undo.run().await,
		}
	}
}
