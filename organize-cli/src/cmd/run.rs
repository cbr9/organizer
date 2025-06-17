use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, ValueHint};

use organize_core::{config::context::RunSettings, engine::Engine};

use crate::Cmd;

use super::logs;

#[derive(Parser, Default, Debug)]
pub struct Run {
	#[arg(long, short = 'c', value_hint = ValueHint::FilePath)]
	config: Option<PathBuf>,
	#[arg(long, conflicts_with = "ids", help = "A space-separated list of tags used to select the rules to be run. To exclude a tag, prefix it with '!'", value_delimiter = ' ', num_args = 1..)]
	tags: Option<Vec<String>>,
	#[arg(long, conflicts_with = "tags", help = "A space-separated list of tags used to filter out rules. To exclude an ID, prefix it with '!'", value_delimiter = ' ', num_args = 1..)]
	ids: Option<Vec<String>>,
	#[arg(long, default_value_t = true, conflicts_with = "no_dry_run")]
	dry_run: bool,
	#[arg(long, conflicts_with = "dry_run")]
	no_dry_run: bool,
	#[arg(long, short = 'v')]
	verbose: bool,
	#[arg(long, help = "Run all operations sequentially in a single thread.")]
	no_parallel: bool,
}

impl Cmd for Run {
	#[tracing::instrument(err)]
	fn run(mut self) -> Result<()> {
		if self.no_dry_run {
			self.dry_run = false;
		}

		let settings = RunSettings {
			dry_run: self.dry_run,
			no_parallel: self.no_parallel,
		};

		let engine = Engine::new(self.config, settings, self.tags, self.ids)?;
		logs::init(self.verbose, &engine.config.path);
		engine.run()?;

		Ok(())
	}
}
