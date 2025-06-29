use std::{
	env,
	path::Path,
	process::{self, ExitStatus},
};

use anyhow::{Context, Result};
use async_trait::async_trait;
use clap::Parser;
use organize_core::config::Config;

use crate::cmd::Cmd;

#[derive(Parser, Debug)]
pub struct Edit;

#[async_trait]
impl Cmd for Edit {
	async fn run(self) -> Result<()> {
		Self::edit(Config::resolve_path(None)).map(|_| ())
	}
}

impl Edit {
	pub(crate) fn edit<T: AsRef<Path>>(path: T) -> Result<ExitStatus> {
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
