use std::path::PathBuf;

use anyhow::Result;
use async_trait::async_trait;
use clap::{Parser, ValueHint};
use organize_core::{context::RunSettings, engine::Engine};

use crate::Cmd;


#[derive(Parser, Default, Debug)]
pub struct Run {
	#[arg(long, short = 'r', value_hint = ValueHint::FilePath)]
	rule: PathBuf,
	#[arg(long, default_value_t = true, conflicts_with = "no_dry_run")]
	dry_run: bool,
	#[arg(long, conflicts_with = "dry_run")]
	no_dry_run: bool,
}

#[async_trait]
impl Cmd for Run {
	async fn run(mut self) -> Result<()> {
		if self.no_dry_run {
			self.dry_run = false;
		}
		let settings = RunSettings { dry_run: self.dry_run };
		let engine = Engine::new(&self.rule, settings).await?;
		engine.run().await?;

		Ok(())
	}
}
