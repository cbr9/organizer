use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use organize_core::{config::Config, file::File};

use crate::Cmd;

#[derive(Parser, Default)]
pub struct RunBuilder {
	#[arg(long, short = 'c')]
	config: Option<PathBuf>,
}

impl RunBuilder {
	pub fn config(mut self, config: Option<PathBuf>) -> Result<Self> {
		self.config = match config {
			Some(config) => Some(config),
			None => Some(Config::path()?),
		};
		Ok(self)
	}
	pub fn build(mut self) -> Result<Run> {
		if self.config.is_none() {
			self = self.config(None)?;
		}
		Ok(Run {
			config: Config::parse(self.config.unwrap()).unwrap(),
		})
	}
}

pub struct Run {
	pub(crate) config: Config,
}

impl Run {
	#[allow(dead_code)]
	pub fn builder() -> RunBuilder {
		RunBuilder::default()
	}
}

impl Cmd for Run {
	fn run(self) -> Result<()> {
		self.start()
	}
}

impl Run {
	pub(crate) fn start(self) -> Result<()> {
		self.config.path_to_rules.iter().for_each(|(path, _)| {
			let recursive = self.config.path_to_recursive.get(path).unwrap();
			let walker = recursive.to_walker(path);
			walker.into_iter().filter_map(|e| e.ok()).for_each(|entry| {
				if entry.path().is_file() {
					let file = File::new(entry.path(), &self.config, false);
					file.act(&self.config.path_to_rules);
				}
			});
		});
		Ok(())
	}
}
