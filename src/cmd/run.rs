use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use organize_core::{config::Config, file::File, logger::Logger};

use crate::Cmd;

#[derive(Parser, Default)]
pub struct RunBuilder {
	#[arg(long, short = 'c')]
	config: Option<PathBuf>,
	#[arg(long)]
	no_color: Option<bool>,
}

impl RunBuilder {
	pub fn config(mut self, config: Option<PathBuf>) -> Result<Self> {
		self.config = match config {
			Some(config) => Some(config),
			None => Some(Config::path()?),
		};
		Ok(self)
	}
	pub fn no_color(mut self, no_color: Option<bool>) -> Self {
		self.no_color = Some(no_color.map_or_else(|| false, |v| !v));
		self
	}
	pub fn build(mut self) -> Result<Run> {
		if self.config.is_none() {
			self = self.config(None)?;
		}
		if self.no_color.is_none() {
			self = self.no_color(None);
		}
		Ok(Run {
			config: Config::parse(self.config.unwrap()).unwrap(),
			no_color: self.no_color.unwrap(),
		})
	}
}

pub struct Run {
	pub(crate) config: Config,
	pub(crate) no_color: bool,
}

impl Run {
	pub fn builder() -> RunBuilder {
		RunBuilder::default()
	}
}

impl Cmd for Run {
	fn run(self) -> Result<()> {
		self.start()
	}
}

impl<'a> Run {
	pub(crate) fn start(self) -> Result<()> {
		Logger::setup(self.no_color)?;
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
