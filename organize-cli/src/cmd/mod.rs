use crate::cmd::run::Run;
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
}

#[async_trait]
pub trait Cmd {
	async fn run(self) -> anyhow::Result<()>;
}

#[async_trait]
impl Cmd for App {
	async fn run(self) -> anyhow::Result<()> {
		match self.command {
			Command::Run(cmd) => cmd.run().await,
			Command::Edit(edit) => edit.run().await,
			Command::Undo(undo) => undo.run().await,
		}
	}
}
