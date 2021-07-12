use std::{
	env,
	path::Path,
	process::{Command, ExitStatus},
};

use anyhow::{Context, Result};
use clap::Clap;

use organize_core::data::{config::Config, settings::Settings};

use crate::cmd::Cmd;
use std::ops::Sub;

#[derive(Clap, Debug)]
pub struct Edit {
	#[clap(subcommand)]
    subcommand: Subcommand
}

// DUMMY ENUM REQUIRED TO AVOID A CLAP BUG
#[derive(Clap, Debug)]
enum Subcommand {
	Config,
	Settings
}

impl Cmd for Edit {
	fn run(self) -> Result<()> {
		match self.subcommand {
			Subcommand::Config => Self::launch_editor(Config::path()?).map(|_| ()),
			Subcommand::Settings => Self::launch_editor(Settings::path()?).map(|_| ()),
		}
	}
}

impl Edit {
	pub(crate) fn launch_editor<T: AsRef<Path>>(path: T) -> Result<ExitStatus> {
		env::var("EDITOR").map(|editor| {
			Command::new(&editor)
				.arg(path.as_ref())
				.spawn()
				.context(editor)?
				.wait()
				.context("command wasn't running")
		})?
	}
}
