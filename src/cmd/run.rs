use std::{
	collections::HashMap,
	iter::FromIterator,
	path::PathBuf,
	sync::{Arc, Mutex},
};

use anyhow::Result;
use clap::{Parser, ValueHint};
use dialoguer::{theme::ColorfulTheme, MultiSelect, Select};
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use std::collections::HashSet;

use organize_core::{
	config::{actions::ActionRunner, filters::AsFilter, options::FolderOptions, rule::Rule, Config, SIMULATION},
	resource::Resource,
	templates::CONTEXT,
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

impl Run {
	fn choose(prompt: &str, items: &[String]) -> Vec<String> {
		let choice = MultiSelect::with_theme(&ColorfulTheme::default())
			.with_prompt(prompt)
			.items(items)
			.interact_opt()
			.unwrap()
			.unwrap_or_default();

		return items
			.iter()
			.enumerate()
			.filter(|(i, _)| choice.contains(i))
			.map(|(_, tag)| tag)
			.cloned()
			.collect();
	}

	fn choose_filters(&mut self, all_tags: &[String], all_ids: &[String]) {
		self.interactive_filter = false;
		let modes = &["Select tags", "Skip tags", "ID"];
		let mode = Select::with_theme(&ColorfulTheme::default())
			.with_prompt("Mode")
			.items(modes)
			.interact_opt()
			.unwrap()
			.unwrap();

		match mode {
			0 => {
				if all_tags.is_empty() {
					println!("There are no rules with an associated tag");
					return;
				}
				self.tags = Some(Self::choose("Tags", all_tags))
			}
			1 => {
				if all_tags.is_empty() {
					println!("There are no rules with an associated tag");
					return;
				}
				self.skip_tags = Some(Self::choose("Skip Tags", all_tags))
			}
			2 => {
				if all_ids.is_empty() {
					println!("There are no rules with an associated ID");
					return;
				}
				self.rules = Some(Self::choose("IDs", all_ids));
			}
			_ => (),
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

		SIMULATION.set(self.dry_run).unwrap();

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
				CONTEXT.lock().unwrap().insert("root", &location.to_string_lossy());
				let walker = FolderOptions::max_depth(config, rule, folder).to_walker(location);

				let mut entries = walker
					.into_iter()
					.flatten()
					.filter(|e| FolderOptions::allows_entry(config, rule, folder, e.path()))
					.map(|e| Resource::new(e.path(), &rule.variables))
					.filter(|e| {
						let mut e = e.clone();
						rule.filters.matches(&mut e)
					})
					.collect::<Vec<_>>();

				entries.par_iter_mut().for_each(|entry| {
					if let Some(last_rule) = processed_files.lock().unwrap().get(entry.path().as_ref()) {
						if !last_rule.r#continue {
							return;
						}
					}
					'actions: for action in rule.actions.iter() {
						match action.run(entry).unwrap() {
							Some(path) => entry.set_path(path),
							None => break 'actions,
						};
					}

					processed_files
						.lock()
						.unwrap()
						.entry(entry.path().as_ref().to_path_buf())
						.and_modify(|value| *value = rule)
						.or_insert(rule);
				})
			}
		}
		Ok(())
	}
}
