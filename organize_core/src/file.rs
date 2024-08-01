use crate::{
	config::{options::r#match::Match, Config},
	path::IsHidden,
};
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
};

pub struct File<'a> {
	pub path: PathBuf,
	config: &'a Config,
}

impl<'a> File<'a> {
	pub fn new<T: Into<PathBuf>>(path: T, config: &'a Config) -> Self {
		Self { path: path.into(), config }
	}

	fn filter_by_recursive<T: AsRef<Path>>(&self, ancestor: T, rule: usize, folder: usize) -> bool {
		let depth = *self.config.get_recursive_depth(rule, folder) as usize;
		if depth == 0 {
			return true;
		}
		return self.path.components().count() - ancestor.as_ref().components().count() <= depth;
	}

	fn filter_by_partial_files(&self, rule: usize, folder: usize) -> bool {
		if !*self.config.allows_partial_files(rule, folder) {
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
		(self.path.is_hidden() && *self.config.allows_hidden_files(rule, folder)) || !self.path.is_hidden()
	}

	fn filter_by_ignored_dirs(&self, rule: usize, folder: usize) -> bool {
		let check_ignored = |dir: &PathBuf| -> bool { self.path.parent().map(|parent| dir == parent).unwrap_or_default() };
		if let Some(ignored_dirs) = &self.config.global_defaults.ignored_dirs {
			if ignored_dirs.iter().any(check_ignored) {
				return false;
			}
		}
		if let Some(ignored_dirs) = &self.config.local_defaults.ignored_dirs {
			if ignored_dirs.iter().any(check_ignored) {
				return false;
			}
		}
		if let Some(ignored_dirs) = &self.config.rules[rule].options.ignored_dirs {
			if ignored_dirs.iter().any(check_ignored) {
				return false;
			}
		}
		if let Some(ignored_dirs) = &self.config.rules[rule].folders[folder].options.ignored_dirs {
			if ignored_dirs.iter().any(check_ignored) {
				return false;
			}
		}
		true
	}

	fn filter_by_options<T: AsRef<Path>>(&self, ancestor: T, rule: usize, folder: usize) -> bool {
		self.filter_by_recursive(ancestor, rule, folder)
			&& self.filter_by_hidden_files(rule, folder)
			&& self.filter_by_ignored_dirs(rule, folder)
			&& self.filter_by_partial_files(rule, folder)
	}

	fn filter_by_filters(&self, rule: usize, folder: usize) -> bool {
		let apply = self.config.get_apply(rule, folder);
		let rule = &self.config.rules[rule];
		rule.filters.r#match(&self.path, apply)
	}

	fn filter<T: AsRef<Path>>(&self, ancestor: T, rule: &usize, folder: &usize) -> bool {
		let (rule, folder) = (*rule, *folder);
		self.filter_by_options(ancestor, rule, folder) && self.filter_by_filters(rule, folder)
	}

	pub fn get_matching_rules(&self, path_to_rules: &'a HashMap<PathBuf, Vec<(usize, usize)>>) -> Vec<&'a (usize, usize)> {
		let (ancestor, rules) = self
			.path
			.ancestors()
			.find_map(|ancestor| path_to_rules.get_key_value(&ancestor.to_path_buf()))
			.unwrap();

		match self.config.match_rules() {
			Match::First => rules
				.iter()
				.find(|(rule, folder)| self.filter(ancestor, rule, folder))
				.map_or_else(Vec::new, |rule| vec![rule]),
			Match::All => rules
				.iter()
				.filter(|(rule, folder)| self.filter(ancestor, rule, folder))
				.collect(),
		}
	}
}
