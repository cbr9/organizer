use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use organize_core::config::{actions::ActionRunner, filters::AsFilter, options::FolderOptions, Config};

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
			Some(ref path) => Config::new(path).expect("Could not parse config"),
			None => Config::new(Config::path().unwrap()).expect("Could not parse config"),
		});

		for rule in config.rules.iter() {
			for folder in rule.folders.iter() {
				let location = folder.path.as_path();
				let walker = FolderOptions::recursive(config, rule, folder).to_walker(location);
				for entry in walker.into_iter() {
					let Ok(entry) = entry else { continue };
					let mut entry = entry.into_path();

					if entry.is_file() && rule.filters.matches(&entry) {
						'actions: for action in rule.actions.iter() {
							match action.run(&entry)? {
								Some(path) => entry = path,
								None => break 'actions,
							};
						}
					}
				}
			}
		}
		Ok(())
	}
}
