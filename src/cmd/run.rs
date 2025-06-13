use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, ValueHint};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use organize_core::config::{options::Options, Config, CONFIG};

use crate::Cmd;

#[derive(Parser, Default)]
pub struct Run {
	#[arg(long, short = 'c', value_hint = ValueHint::FilePath)]
	config: Option<PathBuf>,
	#[arg(long, conflicts_with = "ids", help = "A space-separated list of tags used to select the rules to be run. To exclude a tag, prefix it with '!'", value_delimiter = ' ', num_args = 1..)]
	tags: Option<Vec<String>>,
	#[arg(long, conflicts_with = "tags", help = "A space-separated list of tags used to filter out rules. To exclude an ID, prefix it with '!'", value_delimiter = ' ', num_args = 1..)]
	ids: Option<Vec<String>>,
	#[arg(long, default_value_t = true, conflicts_with = "no_dry_run")]
	dry_run: bool,
	#[arg(long, conflicts_with = "dry_run")]
	no_dry_run: bool,
}

impl Cmd for Run {
	#[tracing::instrument(skip(self))]
	fn run(mut self) -> Result<()> {
		let config = CONFIG.get_or_init(|| match self.config {
			Some(ref path) => Config::new(path).expect("Could not parse config"),
			None => Config::new(Config::path().unwrap()).expect("Could not parse config"),
		});

		let filtered_rules = config.filter_rules(self.tags.as_ref(), self.ids.as_ref());

		if self.no_dry_run {
			self.dry_run = false;
		}

		for rule in filtered_rules.into_iter() {
			let mut entries = rule
				.folders
				.par_iter()
				.map(|folder| Options::get_entries(config, rule, folder).unwrap())
				.flatten()
				.collect::<Vec<_>>();

			entries = rule.filters.filter(entries);

			for action in rule.actions.iter() {
				entries = action.run(entries, self.dry_run);
			}
		}
		Ok(())
	}
}
