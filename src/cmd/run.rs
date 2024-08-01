use std::{path::PathBuf};

use anyhow::Result;
use clap::Parser;

use organize_core::config::{actions::ActionRunner, filters::AsFilter, Config};

use crate::{Cmd, CONFIG};

#[derive(Parser, Default)]
pub struct Run {
	#[arg(long, short = 'c')]
	config: Option<PathBuf>,
}

impl Cmd for Run {
	fn run(self) -> Result<()> {
		self.start()
	}
}

impl Run {
	pub(crate) fn start(self) -> Result<()> {
		let config = CONFIG.get_or_init(|| match self.config {
			Some(ref path) => Config::parse(path).expect("Could not parse config"),
			None => Config::parse(Config::path().unwrap()).expect("Could not parse config"),
		});

		for rule in config.rules.iter() {
			for folder in rule.folders.iter() {
				let location = folder.path.as_path();
				let walker = config.path_to_recursive.get(location).unwrap().to_walker(location).max_depth(1);
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
