use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, ValueHint};
use itertools::Itertools;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};

use organize_core::{
	config::{Config, ConfigBuilder, context::Context},
	templates::TemplateEngine,
};

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
		let config_builder = ConfigBuilder::new(self.config.clone())?;
		let mut engine = TemplateEngine::from_config(&config_builder)?;
		let config = config_builder.build(&mut engine, self.tags, self.ids)?;
		logs::init(self.verbose, &config.path);

		if self.no_dry_run {
			self.dry_run = false;
		}

		for (i, rule) in config.rules.iter().enumerate() {
			for folder in rule.folders.iter() {
				let entries = match folder.get_resources() {
					Ok(entries) => entries,
					Err(e) => {
						tracing::error!(
							"Rule [number = {}, id = {}]: Could not read entries from folder '{}'. Error: {}",
							i,
							rule.id.as_deref().unwrap_or("untitled"),
							folder.path.display(),
							e
						);
						continue;
					}
				};

				let context = Context {
					template_engine: &engine,
					config: &config,
					rule,
					folder,
					dry_run: self.dry_run,
				};

				let filtered_entries = entries
					.into_par_iter()
					.filter(|res| rule.filters.iter().all(|f| f.filter(res, &context)))
					.collect::<Vec<_>>();

				if filtered_entries.is_empty() {
					continue;
				}

				rule.actions
					.iter()
					.fold(filtered_entries, |current_entries, action| action.run(current_entries, &context));
			}
		}
		Ok(())
	}
}
