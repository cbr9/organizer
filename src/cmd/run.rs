use std::{collections::HashMap, iter::FromIterator, path::PathBuf};

use anyhow::Result;
use clap::Parser;

use organize_core::config::{actions::ActionRunner, filters::AsFilter, Config};
use tera::{Context, Tera};

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
		for rule in self.config.rules.iter() {
			for folder in rule.folders.iter() {
				let location = folder.path.as_path();
				let walker = self.config.path_to_recursive.get(location).unwrap().to_walker(location);
				'entries: for entry in walker.into_iter() {
					let Ok(entry) = entry else { continue };
					let mut entry = entry.into_path();

					if entry.is_file() {
						for filter in rule.filters.iter() {
							if !filter.matches(&entry) {
								continue 'entries;
							}
						}
						for action in rule.actions.iter() {
							let res = match action.run(&entry) {
								Ok(path) => path,
								Err(_) => None,
							};
							match res {
								Some(path) => entry = path,
								None => continue 'entries,
							}
						}
					}
				}
			}
		}
		Ok(())
	}
}
