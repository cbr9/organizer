use std::{iter::FromIterator, path::PathBuf};

use anyhow::Result;
use clap::{Parser, ValueHint};
use dialoguer::{theme::ColorfulTheme, MultiSelect, Select};
use std::collections::HashSet;

use organize_core::config::{actions::ActionRunner, filters::AsFilter, options::FolderOptions, rule::Rule, Config};

use crate::{Cmd, CONFIG};

#[derive(Parser, Default)]
pub struct Run {
	#[arg(long, short = 'c', value_hint = ValueHint::FilePath)]
	config: Option<PathBuf>,
	#[arg(long, conflicts_with = "name", help = "A comma-separated list of tags used to select the rules to be run", value_hint = ValueHint::Other, value_parser = parse_comma_separated_values)]
	tags: Option<Vec<String>>,
	#[arg(long, conflicts_with = "name", help = "A comma-separated list of tags used to filter out rules", value_hint = ValueHint::Other, value_parser = parse_comma_separated_values)]
	skip_tags: Option<Vec<String>>,
	#[arg(long, help = "Select specific rules to be run by their IDs", value_hint = ValueHint::Other, value_parser = parse_comma_separated_values)]
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
	fn choose_tags(prompt: &str, all_tags: &Vec<String>) -> Option<Vec<String>> {
		let tags = MultiSelect::with_theme(&ColorfulTheme::default())
			.with_prompt(prompt)
			.items(&all_tags)
			.interact_opt()
			.unwrap()
			.unwrap_or_default();

		return Some(
			all_tags
				.iter()
				.enumerate()
				.filter(|(i, _)| tags.contains(i))
				.map(|(_, tag)| tag)
				.cloned()
				.collect(),
		);
	}

	fn choose_ids(all_ids: &Vec<String>) -> Option<Vec<String>> {
		let ids = MultiSelect::with_theme(&ColorfulTheme::default())
			.with_prompt("Choose rules")
			.items(&all_ids)
			.interact_opt()
			.unwrap()
			.unwrap_or_default();

		return Some(
			all_ids
				.iter()
				.enumerate()
				.filter(|(i, _)| ids.contains(i))
				.map(|(_, tag)| tag)
				.cloned()
				.collect(),
		);
	}

	fn choose_filters(&mut self, all_tags: &Vec<String>, all_ids: &Vec<String>) {
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
				self.tags = Self::choose_tags("Tags", all_tags)
			}
			1 => {
				if all_tags.is_empty() {
					println!("There are no rules with an associated tag");
					return;
				}
				self.skip_tags = Self::choose_tags("Skip Tags", all_tags)
			}
			2 => {
				if all_ids.is_empty() {
					println!("There are no rules with an associated ID");
					return;
				}
				self.rules = Self::choose_ids(all_ids);
			}
			_ => return,
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

		let all_tags: Vec<String> = config.rules.iter().flat_map(|rule| &rule.tags).cloned().collect();
		let all_ids: Vec<String> = config.rules.iter().filter_map(|rule| rule.id.clone()).collect();
		let dry_run = self.dry_run.clone();
		let filtered_rules = config.rules.iter().filter(|rule| self.filter_rules(rule, &all_tags, &all_ids));

		for rule in filtered_rules {
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

				entries.iter_mut().for_each(|entry| {
					for action in rule.actions.iter() {
						let new_path = match action.run(&entry, dry_run) {
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
