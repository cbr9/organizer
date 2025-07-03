use std::{collections::HashMap, path::PathBuf};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use clap::{Parser, ValueHint};
use organize_sdk::{context::settings::RunSettings, engine::Engine};

use crate::Cmd;

#[derive(Parser, Debug)]
pub struct Run {
	#[arg(long, short = 'r', value_hint = ValueHint::FilePath)]
	rule: PathBuf,
	#[arg(long, default_value_t = false)]
	no_dry_run: bool,
	#[arg(last = true, value_parser = parse_key_val)]
	args: Vec<(String, String)>,
}

#[async_trait]
impl Cmd for Run {
	async fn run(mut self) -> Result<()> {
		let settings = RunSettings {
			dry_run: !self.no_dry_run,
			args: self.args.into_iter().collect(),
		};
		let engine = Engine::new(&self.rule, settings).await?;
		engine.run().await?;

		Ok(())
	}
}

fn parse_key_val(s: &str) -> Result<(String, String)> {
	if s.starts_with("--") {
		return Err(anyhow!("invalid argument: {s}, key-value pairs should not start with --"));
	}
	s.split_once('=')
		.map(|(key, value)| (key.to_string(), value.to_string()))
		.ok_or_else(|| anyhow!("invalid key-value pair, please use the format `key=value`"))
}
