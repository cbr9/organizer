use crate::cmd::run::Run;
use async_trait::async_trait;
use clap::{Parser, Subcommand};
use snapshot::Snapshot;
use undo::Undo;

mod logs;
mod run;
mod snapshot;
mod undo;

#[derive(Subcommand)]
enum Command {
	Run(Run),
	Undo(Undo),
	Snapshot(Snapshot),
}

#[derive(Parser)]
#[command(about, author, version)]
pub struct OrganizeCli {
	#[command(subcommand)]
	command: Command,
}

#[async_trait]
pub trait Cmd {
	async fn run(self) -> anyhow::Result<()>;
}

#[async_trait]
impl Cmd for OrganizeCli {
	async fn run(self) -> anyhow::Result<()> {
		let _guard = logs::init();
		match self.command {
			Command::Run(cmd) => cmd.run().await,
			Command::Undo(undo) => undo.run().await,
			Command::Snapshot(snapshot) => snapshot.run().await,
		}
	}
}
