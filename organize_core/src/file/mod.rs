use crate::{
	data::{options::r#match::Match, path_to_rules::PathToRules, Data},
	path::IsHidden,
	simulation::Simulation,
};
use std::{
	path::{Path, PathBuf},
	sync::{Arc, Mutex},
};

pub struct File<'a> {
	pub path: PathBuf,
	data: &'a Data,
	is_watching: bool,
}

impl<'a> File<'a> {
	pub fn new<T: Into<PathBuf>>(path: T, data: &'a Data, is_watching: bool) -> Self {
		Self {
			path: path.into(),
			data,
			is_watching,
		}
	}

	pub fn simulate(mut self, path_to_rules: &'a PathToRules, simulation: &Arc<Mutex<Simulation>>) {
		let rules = self.get_matching_rules(path_to_rules);
		for (i, j) in rules {
			let rule = &self.data.config.rules[*i];
			match rule.actions.simulate(self.path, self.data.get_apply_actions(*i, *j), simulation) {
				None => break,
				Some(new_path) => {
					self.path = new_path;
				}
			}
		}
	}

	pub fn act(mut self, path_to_rules: &'a PathToRules) {
		let rules = self.get_matching_rules(path_to_rules);
		for (i, j) in rules {
			let rule = &self.data.config.rules[*i];
			match rule.actions.act(self.path, self.data.get_apply_actions(*i, *j)) {
				None => break,
				Some(new_path) => {
					self.path = new_path;
				}
			}
		}
	}

	fn filter_by_recursive<T: AsRef<Path>>(&self, ancestor: T, rule: usize, folder: usize) -> bool {
		let depth = *self.data.get_recursive_depth(rule, folder) as usize;
		if depth == 0 {
			return true;
		}
		return self.path.components().count() - ancestor.as_ref().components().count() <= depth;
	}

	fn filter_by_partial_files(&self, rule: usize, folder: usize) -> bool {
		if !*self.data.allows_partial_files(rule, folder) {
			// if partial files are allowed
			if let Some(extension) = self.path.extension() {
				let partial_extensions = &["crdownload", "part"];
				let extension = extension.to_string_lossy();
				return !partial_extensions.contains(&&*extension);
			}
		}
		true
	}

	fn filter_by_hidden_files(&self, rule: usize, folder: usize) -> bool {
		(self.path.is_hidden() && *self.data.allows_hidden_files(rule, folder)) || !self.path.is_hidden()
	}

	fn filter_by_ignored_dirs(&self, rule: usize, folder: usize) -> bool {
		let check_ignored = |dir: &PathBuf| -> bool { self.path.parent().map(|parent| dir == parent).unwrap_or_default() };
		if let Some(ignored_dirs) = &self.data.settings.defaults.ignored_dirs {
			if ignored_dirs.iter().any(check_ignored) {
				return false;
			}
		}
		if let Some(ignored_dirs) = &self.data.config.defaults.ignored_dirs {
			if ignored_dirs.iter().any(check_ignored) {
				return false;
			}
		}
		if let Some(ignored_dirs) = &self.data.config.rules[rule].options.ignored_dirs {
			if ignored_dirs.iter().any(check_ignored) {
				return false;
			}
		}
		if let Some(ignored_dirs) = &self.data.config.rules[rule].folders[folder].options.ignored_dirs {
			if ignored_dirs.iter().any(check_ignored) {
				return false;
			}
		}
		true
	}

	fn filter_by_watch(&self, rule: usize, folder: usize) -> bool {
		!self.is_watching || *self.data.allows_watching(rule, folder)
	}

	fn filter_by_options<T: AsRef<Path>>(&self, ancestor: T, rule: usize, folder: usize) -> bool {
		self.filter_by_recursive(ancestor, rule, folder)
			&& self.filter_by_hidden_files(rule, folder)
			&& self.filter_by_ignored_dirs(rule, folder)
			&& self.filter_by_partial_files(rule, folder)
			&& self.filter_by_watch(rule, folder)
	}

	fn filter_by_filters(&self, rule: usize, folder: usize) -> bool {
		let apply = self.data.get_apply_filters(rule, folder);
		let rule = &self.data.config.rules[rule];
		rule.filters.r#match(&self.path, apply)
	}

	fn filter<T: AsRef<Path>>(&self, ancestor: T, rule: &usize, folder: &usize) -> bool {
		let (rule, folder) = (*rule, *folder);
		self.filter_by_options(ancestor, rule, folder) && self.filter_by_filters(rule, folder)
	}

	pub fn get_matching_rules(&mut self, path_to_rules: &'a PathToRules) -> Vec<&'a (usize, usize)> {
		let (ancestor, rules) = path_to_rules.get_key_value(&self.path).unwrap();
		match self.data.match_rules() {
			Match::First => rules
				.iter()
				.find(|(rule, folder)| self.filter(ancestor, rule, folder))
				.map_or_else(Vec::new, |rule| vec![rule]),
			Match::All => rules.iter().filter(|(rule, folder)| self.filter(ancestor, rule, folder)).collect(),
		}
	}
}
