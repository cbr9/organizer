use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, ValueHint};
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};

use organize_core::config::Config;

use crate::Cmd;

use super::logs;

#[derive(Parser, Default, Debug)]
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
	#[arg(long, short = 'v')]
	verbose: bool,
}

impl Cmd for Run {
	#[tracing::instrument(err)]
	fn run(mut self) -> Result<()> {
		let config = Config::new(self.config.clone())?;
		logs::init(self.verbose, &config.path);

		let filtered_rules = config.filter_rules(self.tags.as_ref(), self.ids.as_ref());

		if self.no_dry_run {
			self.dry_run = false;
		}

		for (i, rule) in filtered_rules.into_iter().enumerate() {
			let entries = rule
				.folders
				.par_iter()
				.filter_map(|folder| {
					folder
						.get_resources()
						.inspect_err(|e| {
							tracing::error!(
								"Rule [number = {}, id = {}]: Could not read entries from folder '{}'. Error: {}",
								i,
								rule.id.as_deref().unwrap_or("untitled"),
								folder.path.display(),
								e
							)
						})
						.ok()
				})
				.flatten()
				.into_par_iter()
				.filter(|res| {
					rule.filters
						.iter()
						.all(|f| f.filter(res, &rule.template_engine, &rule.variables))
				})
				.collect::<Vec<_>>();

			rule.actions.iter().fold(entries, |current_entries, action| {
				action.run(current_entries, &rule.template_engine, &rule.variables, self.dry_run)
			});
		}
		Ok(())
	}
}
