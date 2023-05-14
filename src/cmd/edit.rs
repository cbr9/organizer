use std::{
	env,
	path::Path,
	process::{self, ExitStatus},
};

use anyhow::{Context, Result};
use clap::Parser;

use organize_core::config::Config;

use crate::cmd::Cmd;

#[derive(Parser, Debug)]
pub struct Edit;

impl Cmd for Edit {
	fn run(self) -> Result<()> {
		Self::launch_editor(Config::path()?).map(|_| ())
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
