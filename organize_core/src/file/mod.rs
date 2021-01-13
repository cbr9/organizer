use crate::data::path_to_recursive::PathToRecursive;
use crate::data::{options::r#match::Match, path_to_rules::PathToRules, Data};
use crate::simulation::Simulation;
use notify::RecursiveMode;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct File {
	pub path: PathBuf,
}

impl File {
	pub fn new<T: Into<PathBuf>>(path: T) -> Self {
		Self { path: path.into() }
	}
	pub fn simulate<'a>(
		self,
		data: &'a Data,
		path_to_rules: &'a PathToRules,
		path_to_recursive: &'a PathToRecursive,
		simulation: &Arc<Mutex<Simulation>>,
	) {
		let mut path = self.path.clone();
		match data.get_match() {
			Match::All => {
				let rules = self.get_matching_rules(data, path_to_rules, path_to_recursive);
				for (i, j) in rules {
					let rule = &data.config.rules[*i];
					match rule.actions.simulate(&path, data.get_apply_actions(*i, *j), simulation) {
						None => break,
						Some(new_path) => {
							path = new_path;
						}
					}
				}
			}
			Match::First => {
				let rules = self.get_matching_rules(data, path_to_rules, path_to_recursive);
				if let Some((i, j)) = rules.first() {
					let rule = &data.config.rules[*i];
					rule.actions.simulate(&path, data.get_apply_actions(*i, *j), simulation);
				}
			}
		}
	}

	pub fn act<'a>(self, data: &'a Data, path_to_rules: &'a PathToRules, path_to_recursive: &'a PathToRecursive) {
		let mut path = self.path.clone();
		match data.get_match() {
			Match::All => {
				let rules = self.get_matching_rules(data, path_to_rules, path_to_recursive);
				for (i, j) in rules {
					let rule = &data.config.rules[*i];
					match rule.actions.act(&path, data.get_apply_actions(*i, *j)) {
						None => break,
						Some(new_path) => {
							path = new_path;
						}
					}
				}
			}
			Match::First => {
				let rules = self.get_matching_rules(data, path_to_rules, path_to_recursive);
				if let Some((i, j)) = rules.first() {
					let rule = &data.config.rules[*i];
					rule.actions.act(&path, data.get_apply_actions(*i, *j));
				}
			}
		}
	}

	pub fn get_matching_rules<'a>(
		&self,
		data: &'a Data,
		path_to_rules: &'a PathToRules,
		path_to_recursive: &'a PathToRecursive,
	) -> Vec<&'a (usize, usize)> {
		let (key, value) = path_to_rules.get_key_value(&self.path);
		let (recursive, depth) = path_to_recursive.get(&key).unwrap();
		if recursive == &RecursiveMode::Recursive {
			let depth = depth.expect("folder is recursive but depth is not defined") as usize;
			if self.path.components().count() - key.components().count() > depth && depth != 0 {
				return Vec::with_capacity(0);
			}
		}
		value
			.iter()
			.filter(|(i, j)| {
				!data.should_ignore(&self.path, *i, *j) && data.config.rules[*i].filters.r#match(&self.path, data.get_apply_filters(*i, *j))
			})
			.collect::<Vec<_>>()
	}
}
