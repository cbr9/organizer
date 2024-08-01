use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use rayon::prelude::*;

use organize_core::config::{actions::ActionRunner, filters::AsFilter, options::FolderOptions, Config};

use crate::{Cmd, CONFIG};

#[derive(Parser, Default)]
pub struct Run {
	#[arg(long, short = 'c')]
	config: Option<PathBuf>,
	#[arg(long)]
	dry_run: bool,
}

impl Cmd for Run {
	fn run(self) -> Result<()> {
		let config = CONFIG.get_or_init(|| match self.config {
			Some(ref path) => Config::new(path).expect("Could not parse config"),
			None => Config::new(Config::path().unwrap()).expect("Could not parse config"),
		});

		for rule in config.rules.iter() {
			for folder in rule.folders.iter() {
				let location = folder.path.as_path();
				let walker = FolderOptions::max_depth(config, rule, folder)
					.to_walker(location)
					.sort_by_file_name();

				let mut entries = walker
					.into_iter()
					.filter_entry(|e| FolderOptions::allows_entry(config, rule, folder, e))
					.filter_map(|e| e.ok())
					.map(|e| e.into_path())
					.collect::<Vec<_>>();

				entries.par_iter_mut().for_each(|entry| {
					if entry.is_file() && rule.filters.matches(&entry) {
						'actions: for action in rule.actions.iter() {
							let new_path = match action.run(&entry, self.dry_run) {
								Ok(path) => path,
								Err(e) => {
									log::error!("{}", e);
									None
								}
							};
							match new_path {
								Some(path) => *entry = path,
								None => break 'actions,
							};
						}
					}
				})
			}
		}
		Ok(())
	}
}
