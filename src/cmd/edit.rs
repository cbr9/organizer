use std::env;
use std::path::Path;
use std::process::{Command, ExitStatus};

use anyhow::{Context, Result};
use clap::Clap;

use organize_core::data::config::Config;
use organize_core::data::settings::Settings;

use crate::cmd::Cmd;

#[derive(Clap, Debug)]
pub enum Edit {
	Config,
	Settings,
}

impl Cmd for Edit {
	fn run(self) -> Result<()> {
		match self {
			Edit::Config => Self::launch_editor(Config::path()?).map(|_| ()),
			Edit::Settings => Self::launch_editor(Settings::path()).map(|_| ()),
		}
	}
}

impl Edit {
	pub(crate) fn launch_editor<T: AsRef<Path>>(path: T) -> anyhow::Result<ExitStatus> {
		Ok(env::var("EDITOR").map(|editor| {
			let mut command = Command::new(&editor);
			command
				.arg(path.as_ref())
				.spawn()
				.context(editor)?
				.wait()
				.context("command wasn't running")
		})??)
	}
}
