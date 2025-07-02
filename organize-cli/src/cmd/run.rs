use std::path::PathBuf;

use anyhow::Result;
use async_trait::async_trait;
use clap::{Parser, ValueHint};
use organize_sdk::{context::RunSettings, engine::Engine};

use crate::Cmd;

#[derive(Parser, Default, Debug)]
pub struct Run {
	#[arg(long, short = 'r', value_hint = ValueHint::FilePath)]
	rule: PathBuf,
	#[arg(long, default_value_t = false)]
	no_dry_run: bool,
}

#[async_trait]
impl Cmd for Run {
	async fn run(mut self) -> Result<()> {
		let settings = RunSettings { dry_run: !self.no_dry_run };
		let engine = Engine::new(&self.rule, settings).await?;
		engine.run().await?;

		Ok(())
	}
}
