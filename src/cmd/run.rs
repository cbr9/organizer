use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use rayon::prelude::*;
use std::collections::HashSet;

use organize_core::config::{actions::ActionRunner, filters::AsFilter, options::FolderOptions, rule::Rule, Config};

use crate::{Cmd, CONFIG};

#[derive(Parser, Default)]
pub struct Run {
	#[arg(long, short = 'c')]
	config: Option<PathBuf>,
	#[arg(long, short = 't')]
	tags: Option<String>,
	#[arg(long)]
	skip_tags: Option<String>,
	#[arg(long)]
	dry_run: bool,
}

impl Run {
	fn filter_rules(&self, rule: &Rule) -> bool {
		let chosen_tags = self.tags.clone().unwrap_or_default();
		let skipped_tags = self.skip_tags.clone().unwrap_or_default();
		let chosen_tags: HashSet<&str> = chosen_tags.split(',').collect();
		let skipped_tags: HashSet<&str> = skipped_tags.split(',').collect();
		rule.tags.iter().all(|tag| {
			if tag == "always" {
				return !skipped_tags.contains(tag.as_str());
			}

			if tag == "never" {
				return chosen_tags.contains(tag.as_str());
			}

			chosen_tags
				.difference(&skipped_tags)
				.map(|s| s.to_string())
				.collect::<HashSet<String>>()
				.contains(tag)
		})
	}
}

impl Cmd for Run {
	fn run(self) -> Result<()> {
		let config = CONFIG.get_or_init(|| match self.config {
			Some(ref path) => Config::new(path).expect("Could not parse config"),
			None => Config::new(Config::path().unwrap()).expect("Could not parse config"),
		});

		for rule in config.rules.iter().filter(|rule| self.filter_rules(rule)) {
			for folder in rule.folders.iter() {
				let location = folder.path.as_path();
				let walker = FolderOptions::max_depth(config, rule, folder)
					.to_walker(location)
					.sort_by_file_name();

				let mut entries = walker
					.into_iter()
					.filter_entry(|e| {
						let path = e.path();
						path.is_file() && FolderOptions::allows_entry(config, rule, folder, path) && rule.filters.matches(path)
					})
					.filter_map(|e| e.ok())
					.map(|e| e.into_path())
					.collect::<Vec<_>>();

				entries.par_iter_mut().for_each(|entry| {
					for action in rule.actions.iter() {
						let new_path = match action.run(&entry, self.dry_run) {
							Ok(path) => path,
							Err(e) => {
								log::error!("{}", e);
								None
							}
						};
						match new_path {
							Some(path) => *entry = path,
							None => break,
						};
					}
				})
			}
		}
		Ok(())
	}
}
