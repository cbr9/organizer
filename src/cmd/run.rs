use std::{
	collections::HashMap,
	path::PathBuf,
	sync::{Arc, Mutex},
};

use anyhow::Result;
use clap::{Parser, ValueHint};
use log::error;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};

use organize_core::{
	config::{actions::ActionPipeline, filters::AsFilter, options::Options, rule::Rule, Config},
	resource::Resource,
};

use crate::{Cmd, CONFIG};

#[derive(Parser, Default)]
pub struct Run {
	#[arg(long, short = 'c', value_hint = ValueHint::FilePath)]
	config: Option<PathBuf>,
	#[arg(long, conflicts_with = "ids", help = "A space-separated list of tags used to select the rules to be run. To exclude a tag, prefix it with '!'", value_delimiter = ' ', num_args = 1..)]
	tags: Option<Vec<String>>,
	#[arg(long, conflicts_with = "tags", help = "A space-separated list of tags used to filter out rules. To exclude an ID, prefix it with '!'", value_delimiter = ' ', num_args = 1..)]
	ids: Option<Vec<String>>,
	#[arg(long)]
	dry_run: bool,
}

impl Cmd for Run {
	fn run(self) -> Result<()> {
		let config = CONFIG.get_or_init(|| match self.config {
			Some(ref path) => Config::new(path).expect("Could not parse config"),
			None => Config::new(Config::path().unwrap()).expect("Could not parse config"),
		});

		let processed_files: Arc<Mutex<HashMap<PathBuf, &Rule>>> = Arc::new(Mutex::new(HashMap::new()));
		let filtered_rules = config.filter_rules(self.tags.as_ref(), self.ids.as_ref());

		for rule in filtered_rules.iter() {
			processed_files.lock().unwrap().retain(|key, _| key.exists());
			for folder in rule.folders.iter() {
				let location = folder.path()?;
				let walker = Options::walker(config, rule, folder)?;

				let mut entries = walker
					.into_iter()
					.filter_entry(|e| Options::prefilter(config, rule, folder, e.path()))
					.flatten()
					.map(|e| Resource::new(e.path(), &location, &rule.variables))
					.filter(|e| rule.filters.matches(e))
					.filter(|e| Options::postfilter(config, rule, folder, &e.path))
					.collect::<Vec<_>>();

				entries.par_iter_mut().for_each(|entry| {
					if let Some(last_rule) = processed_files.lock().unwrap().get(&entry.path) {
						if !last_rule.r#continue {
							return;
						}
					}

					'actions: for action in rule.actions.iter() {
						let path = match action.run(entry, self.dry_run) {
							Ok(path) => path,
							Err(e) => {
								error!("{}", e);
								None
							}
						};

						match path {
							Some(path) => entry.set_path(path),
							None => break 'actions,
						};
					}

					processed_files
						.lock()
						.unwrap()
						.entry(entry.path.clone())
						.and_modify(|value| *value = rule)
						.or_insert(rule);
				})
			}
		}
		Ok(())
	}
}
