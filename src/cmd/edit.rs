use std::{
	env,
	path::Path,
	process::{self, ExitStatus},
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use organize_core::data::{config::Config, settings::Settings};

use crate::cmd::Cmd;

#[derive(Parser, Debug)]
pub struct Edit {
	#[command(subcommand)]
	command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
	Config,
	Settings,
}

impl Cmd for Edit {
	fn run(self) -> Result<()> {
		match self.command {
			Command::Config => Self::launch_editor(Config::path()?).map(|_| ()),
			Command::Settings => Self::launch_editor(Settings::path()?).map(|_| ()),
		}
	}
}

impl Edit {
	pub(crate) fn launch_editor<T: AsRef<Path>>(path: T) -> Result<ExitStatus> {
		env::var("EDITOR").map(|editor| {
			process::Command::new(&editor)
				.arg(path.as_ref())
				.spawn()
				.context(editor)?
				.wait()
				.context("command wasn't running")
		})?
	}
}
