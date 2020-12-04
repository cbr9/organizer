use crate::cmd::Cmd;
use anyhow::{Context, Result};
use clap::{Clap};

use organize_core::{
	data::{
		config::UserConfig,
		options::{Options},
	},
};
use std::process::{Command, ExitStatus};

use std::{env};
use std::path::Path;
use organize_core::data::settings::Settings;

#[derive(Clap, Debug)]
pub enum Edit {
	Config,
	Settings
}

impl Cmd for Edit {
	fn run(self) -> Result<()> {
        match self {
			Edit::Config => Self::launch_editor(UserConfig::path()).map(|_| ()),
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
				.context(format!("{}", &editor))?
				.wait()
				.context("command wasn't running")
		})??)
	}
}