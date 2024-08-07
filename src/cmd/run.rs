use std::{
	collections::HashMap,
	iter::FromIterator,
	path::PathBuf,
	sync::{Arc, Mutex},
};

use anyhow::Result;
use clap::{Parser, ValueHint};
use dialoguer::{theme::ColorfulTheme, MultiSelect, Select};
use log::error;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use std::collections::HashSet;
use strum::{AsRefStr, VariantNames};

use organize_core::{
	config::{actions::ActionPipeline, filters::AsFilter, options::Options, rule::Rule, Config},
	resource::Resource,
};

use crate::{Cmd, CONFIG};

#[derive(Parser, Default)]
pub struct Run {
	#[arg(long, short = 'c', value_hint = ValueHint::FilePath)]
	config: Option<PathBuf>,
	#[arg(long, conflicts_with = "rules", help = "A comma-separated list of tags used to select the rules to be run", value_parser = parse_comma_separated_values)]
	tags: Option<Vec<String>>,
	#[arg(long, conflicts_with = "rules", help = "A comma-separated list of tags used to filter out rules", value_parser = parse_comma_separated_values)]
	skip_tags: Option<Vec<String>>,
	#[arg(long, help = "Select specific rules to be run by their IDs", value_parser = parse_comma_separated_values)]
	rules: Option<Vec<String>>,
	#[arg(long, short = 'i', conflicts_with_all = ["rules", "tags", "skip_tags"], help = "Filter out rules in an interactive way")]
	interactive_filter: bool,
	#[arg(long)]
	dry_run: bool,
}

fn parse_comma_separated_values(s: &str) -> Result<Vec<String>, String> {
	let values = s
		.split(',')
		.map(str::trim)
		.filter(|s| !s.is_empty())
		.map(String::from)
		.collect();
	Ok(values)
}

#[derive(Debug, VariantNames, AsRefStr)]
enum InteractiveChoice {
	Tags(Vec<String>),
	#[strum(serialize = "Skip Tags")]
	SkipTags(Vec<String>),
	IDs(Vec<String>),
}

impl InteractiveChoice {
	fn choose(&self) -> Option<Vec<String>> {
		let items = match self {
			InteractiveChoice::Tags(tags) | InteractiveChoice::SkipTags(tags) => {
				if tags.is_empty() {
					println!("There are no rules with an associated tag");
					return None;
				}
				tags
			}
			InteractiveChoice::IDs(ids) => {
				if ids.is_empty() {
					println!("There are no rules with an associated ID");
					return None;
				}
				ids
			}
		};

		let choice = MultiSelect::with_theme(&ColorfulTheme::default())
			.with_prompt(self.as_ref())
			.items(items)
			.interact_opt()
			.unwrap()
			.unwrap_or_default();

		return Some(
			items
				.iter()
				.enumerate()
				.filter(|(i, _)| choice.contains(i))
				.map(|(_, tag)| tag)
				.cloned()
				.collect(),
		);
	}
}

impl From<usize> for InteractiveChoice {
	fn from(value: usize) -> Self {
		match value {
			0 => Self::Tags(vec![]),
			1 => Self::SkipTags(vec![]),
			2 => Self::IDs(vec![]),
			_ => unimplemented!(),
		}
	}
}

impl Run {
	fn choose_filters(&mut self, all_tags: &[String], all_ids: &[String]) {
		self.interactive_filter = false;
		let chooser = Select::with_theme(&ColorfulTheme::default())
			.with_prompt("Mode")
			.items(InteractiveChoice::VARIANTS)
			.interact_opt()
			.unwrap()
			.map(|u| {
				let mut choice = InteractiveChoice::from(u);
				match choice {
					InteractiveChoice::Tags(ref mut v) | InteractiveChoice::SkipTags(ref mut v) => *v = all_tags.into(),
					InteractiveChoice::IDs(ref mut v) => *v = all_ids.into(),
				};
				choice
			})
			.unwrap();

		match chooser {
			InteractiveChoice::Tags(_) => self.tags = chooser.choose(),
			InteractiveChoice::SkipTags(_) => self.skip_tags = chooser.choose(),
			InteractiveChoice::IDs(_) => self.rules = chooser.choose(),
		}
	}

	fn filter_rules(&mut self, rule: &Rule, all_tags: &Vec<String>, all_ids: &Vec<String>) -> bool {
		if let Some(ids) = self.rules.as_ref() {
			return rule.id.as_ref().is_some_and(|id| ids.contains(id));
		} else if self.tags.is_some() || self.skip_tags.is_some() {
			let chosen_tags: HashSet<String> = HashSet::from_iter(self.tags.clone().unwrap_or_default());
			let skipped_tags = HashSet::from_iter(self.skip_tags.clone().unwrap_or_default());
			return rule.tags.iter().any(|tag| {
				if tag == "always" {
					return !skipped_tags.contains(tag);
				}

				if tag == "never" {
					return chosen_tags.contains(tag);
				}

				return chosen_tags
					.difference(&skipped_tags)
					.map(|s| s.to_string())
					.collect::<HashSet<String>>()
					.contains(tag);
			});
		} else if self.interactive_filter {
			self.choose_filters(all_tags, all_ids);
			return self.filter_rules(rule, all_tags, all_ids);
		}
		true
	}
}

impl Cmd for Run {
	fn run(mut self) -> Result<()> {
		let config = CONFIG.get_or_init(|| match self.config {
			Some(ref path) => Config::new(path).expect("Could not parse config"),
			None => Config::new(Config::path().unwrap()).expect("Could not parse config"),
		});

		let processed_files: Arc<Mutex<HashMap<PathBuf, &Rule>>> = Arc::new(Mutex::new(HashMap::new()));
		let all_tags: Vec<String> = config.rules.iter().flat_map(|rule| &rule.tags).cloned().collect();
		let all_ids: Vec<String> = config.rules.iter().filter_map(|rule| rule.id.clone()).collect();
		let filtered_rules: Vec<Rule> = config
			.rules
			.iter()
			.filter(|rule| self.filter_rules(rule, &all_tags, &all_ids))
			.cloned()
			.collect();

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
